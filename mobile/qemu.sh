#!/usr/bin/env bash

# apt install -y qemu-system-x86 qemu-utils qemu-block-extra

# this is qemu guest management build script for running android build in qemu guest.

show_usage() {
  echo "Usage: qemu.sh <command>"
  echo ""
  echo "Commands:"
  echo "  init       - host initialization if qcow2 image not exists"
  echo "  rundebug   - run qemu with existing qcow2 image on foreground"
  echo "  run        - run qemu with existing qcow2 image on background"
  echo "  stop       - stop the running qemu guest"
  echo "  status     - check if qemu guest is running"
  echo "  reset      - delete qcow2 and unattended ISO for fresh rebuild"
  echo "  ssh        - ssh into the qemu guest"
  echo "  syncto     - rsync project from host to guest"
  echo "  syncfrom   - rsync build outputs from guest to host"
  echo "  install    - adb install debug app"
  echo "  slog       - adb logcat filter single app"
  echo "  glog       - adb logcat grep single app"
  echo "  help       - show this usage"
}

if [[ "$1" == "help" ]] || [[ -z "${1:-}" ]]; then
  show_usage
  exit 0
fi

# variables
PROJECT_NAME="uad-shizuku"
PROJECT_ROOT=$(realpath ..)
RELATIVE_DIR="mobile"
CONTAINER_DIR="/opt"
TAKEOUT_DIR="app/build/outputs"

set -e

# make qemu working dir
mkdir -p ~/qemu

# check if qemu is already running
check_qemu_running() {
  if pgrep -f "qemu-system-x86_64.*alpine.qcow2" > /dev/null; then
    return 0  # running
  else
    return 1  # not running
  fi
}

# init mode
if [[ "$1" == "init" ]]; then
  # Check if QEMU is already running
  if check_qemu_running; then
    echo "ERROR: QEMU guest is already running!"
    echo "Please stop it first with: ./qemu.sh stop"
    echo "Or check status with: ./qemu.sh status"
    exit 1
  fi
  pushd ~/qemu
    # download installer image (extended version includes syslinux)
    if [[ ! -f ~/qemu/alpine-extended-3.23.3-x86_64.iso ]]; then
      echo "=== downloading alpine extended iso ==="
      wget https://dl-cdn.alpinelinux.org/alpine/v3.23/releases/x86_64/alpine-extended-3.23.3-x86_64.iso
    else
      echo "=== alpine iso already exists, skipping download ==="
    fi

    # edit iso file for unattended installation
    # https://www.skreutz.com/posts/unattended-installation-of-alpine-linux/
    if [[ ! -f ~/qemu/alpine-unattended.iso ]]; then
      echo "=== creating unattended installation overlay ==="

      # Create overlay directory structure
      mkdir -p ovl/etc/auto-setup-alpine
      mkdir -p ovl/etc/local.d
      mkdir -p ovl/etc/runlevels/default
      mkdir -p ovl/etc/apk

      # Enable default boot services
      touch ovl/etc/.default_boot_services

      # Enable the local service
      ln -sf /etc/init.d/local ovl/etc/runlevels/default/local

      # Configure APK repositories
      cat > ovl/etc/apk/repositories <<'EOF'
/media/cdrom/apks
# https://dl-cdn.alpinelinux.org/alpine/v3.23/main
# https://dl-cdn.alpinelinux.org/alpine/v3.23/community
EOF

      # generate ed25519 ssh key for root and get its public key
      echo "=== Setting up SSH keys ==="
      mkdir -p ovl/root/.ssh .qemu_ssh
      if [[ ! -f .qemu_ssh/id_ed25519 ]]; then
        ssh-keygen -t ed25519 -f .qemu_ssh/id_ed25519 -N '' -C 'root@alpine'
        echo "=== SSH key pair generated ==="
      else
        echo "=== Using existing SSH key pair ==="
      fi

      # Copy public key to overlay authorized_keys
      cp .qemu_ssh/id_ed25519.pub ovl/root/.ssh/authorized_keys
      chmod 600 ovl/root/.ssh/authorized_keys
      chmod 700 ovl/root/.ssh

      echo "=== Public key added to overlay ==="
      cat ovl/root/.ssh/authorized_keys

      # Create answers file for setup-alpine
      cat > ovl/etc/auto-setup-alpine/answers <<'EOF'
KEYMAPOPTS=none
HOSTNAMEOPTS=alpine
DEVDOPTS=mdev

TIMEZONEOPTS="-z UTC"
PROXYOPTS=none
APKREPOSOPTS="-1"
NTPOPTS="openntpd"

# System installation (use disk)
DISKOPTS="-m sys /dev/sda"
# Setup storage with label APKOVL for config storage
#LBUOPTS="LABEL=APKOVL"
LBUOPTS=none

# Admin user - change this or remove USEROPTS/USERSSHKEY if not needed
USEROPTS="-a -u -g audio,input,video,netdev admin"
# USERSSHKEY="ssh-rsa YOUR_SSH_KEY admin@localhost"
#USERSSHKEY="https://example.com/juser.keys"

# Install Openssh
SSHDOPTS=openssh
EOF

      ROOTSSHKEY="$(cat .qemu_ssh/id_ed25519.pub)"
      cat >> ovl/etc/auto-setup-alpine/answers <<EOF
ROOTSSHKEY="${ROOTSSHKEY}"
EOF

      cat >> ovl/etc/auto-setup-alpine/answers <<'EOF'

# Contents of /etc/network/interfaces
INTERFACESOPTS="auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
"
EOF

      # Create After Install Script to setup ssh-server
      cat > ovl/etc/auto-setup-alpine/after-install.start <<'EOF'
#!/bin/sh
#
echo "=== Running after-install configuration ==="

# Set dns server
echo "nameserver 1.1.1.1" > /etc/resolv.conf
chmod 444 /etc/resolv.conf

# Change Debuggable Message
# Configure bootloader
sed -i 's/quiet//g; s/^default_kernel_opts="\(.*\)"/default_kernel_opts="\1 console=tty0 console=ttyS0,115200"/; s/  */ /g' /etc/update-extlinux.conf
update-extlinux
# Configure OpenRC
grep -q "^rc_verbose=" /etc/rc.conf && sed -i 's/^rc_verbose=.*/rc_verbose=yes/' /etc/rc.conf || echo 'rc_verbose=yes' >> /etc/rc.conf
grep -q "^rc_logger=" /etc/rc.conf && sed -i 's/^rc_logger=.*/rc_logger=YES/' /etc/rc.conf || echo 'rc_logger=YES' >> /etc/rc.conf

# Configure APK repositories
cat > /etc/apk/repositories <<'REPOS_EOF'
https://dl-cdn.alpinelinux.org/alpine/v3.23/main
https://dl-cdn.alpinelinux.org/alpine/v3.23/community
REPOS_EOF

# Update and upgrade packages
apk update
apk upgrade

# Install useful packages
apk add bash curl wget git rsync openssh-server

# Configure SSH server (use sed to modify, not append)
sed -i 's/^#*PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config
sed -i 's/^#*PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config
sed -i 's/^#*PubkeyAuthentication.*/PubkeyAuthentication yes/' /etc/ssh/sshd_config

# Fix root permission for SSH
chmod 755 /root

# Ensure SSH directory and keys have correct permissions
mkdir -p /root/.ssh
chmod 700 /root/.ssh
if [ -f /root/.ssh/authorized_keys ]; then
  chmod 600 /root/.ssh/authorized_keys
  echo "=== SSH authorized_keys configured ==="
  ls -la /root/.ssh/
else
  echo "=== WARNING: authorized_keys not found ==="
fi

# Generate host keys if they don't exist
ssh-keygen -A

# Enable SSH server
rc-update add sshd default
rc-service sshd start

echo "=== After-install configuration complete ==="
# Remove this script after first run
rm -f /etc/local.d/after-install.start

EOF
      # Create auto-setup script
      cat > ovl/etc/local.d/auto-setup-alpine.start <<'EOF'
#!/bin/sh

set -o errexit
set -o nounset

# Uncomment to shutdown on completion
trap 'poweroff' EXIT INT

# Close standard input
exec 0<&-

# Run only once
rm -f /etc/local.d/auto-setup-alpine.start
rm -f /etc/runlevels/default/local

# Run setup-alpine with answers file (it will use the prepared partition)
yes yes | timeout 600 setup-alpine -ef /etc/auto-setup-alpine/answers

# Mount the new system and configure it
mount /dev/sda3 /mnt

# Change Dns server
echo "nameserver 1.1.1.1" > /mnt/etc/resolv.conf
chmod 444 /mnt/etc/resolv.conf

# Configure OpenRC
grep -q "^rc_verbose=" /mnt/etc/rc.conf && sed -i 's/^rc_verbose=.*/rc_verbose=yes/' /mnt/etc/rc.conf || echo 'rc_verbose=yes' >> /mnt/etc/rc.conf
grep -q "^rc_logger=" /mnt/etc/rc.conf && sed -i 's/^rc_logger=.*/rc_logger=YES/' /mnt/etc/rc.conf || echo 'rc_logger=YES' >> /mnt/etc/rc.conf

# Copy SSH keys from overlay to installed system
echo "=== Configuring SSH keys for installed system ==="
mkdir -p /mnt/root/.ssh
if [ -f /root/.ssh/authorized_keys ]; then
  cp /root/.ssh/authorized_keys /mnt/root/.ssh/authorized_keys
  chmod 700 /mnt/root/.ssh
  chmod 600 /mnt/root/.ssh/authorized_keys
  echo "=== SSH keys copied successfully ==="
  ls -la /mnt/root/.ssh/
else
  echo "=== WARNING: Source authorized_keys not found ==="
fi

# Set root password for fallback access
echo "root:root" | chroot /mnt /usr/sbin/chpasswd
echo "=== Root password set to 'root' ==="

# Copy after-install script and run it in chroot
cp /etc/auto-setup-alpine/after-install.start /mnt/etc/local.d/after-install.start
chmod +x /mnt/etc/local.d/after-install.start

# register to run local service at boot
ln -s /etc/init.d/local /mnt/etc/runlevels/default/local

rm -rf /etc/auto-setup-alpine

# Shutdown after installation
poweroff

EOF

      # Make script executable
      chmod 755 ovl/etc/local.d/auto-setup-alpine.start

      # Create overlay tarball
      echo "=== creating overlay tarball ==="
      tar --owner=0 --group=0 -czf alpine.apkovl.tar.gz -C ovl .

      # Add overlay to ISO image using xorriso
      echo "=== creating unattended installation ISO ==="
      if command -v xorriso &> /dev/null; then
        # Create boot configuration to auto-load the overlay
        mkdir -p isolinux
        cat > isolinux/isolinux.cfg <<'ISOLINUX_EOF'
TIMEOUT 1
DEFAULT lts
PROMPT 0

LABEL lts
  KERNEL /boot/vmlinuz-lts
  INITRD /boot/initramfs-lts
  APPEND modloop=/boot/modloop-lts console=ttyS0 apkovl=/alpine.apkovl.tar.gz
ISOLINUX_EOF
        cat > install.sh <<'INSTALL_EOF'
#!/usr/bin/env sh
tar zxvf /media/cdrom/alpine.apkovl.tar.gz -C /
/etc/local.d/auto-setup-alpine.start

echo " === this is post installation process === "
INSTALL_EOF

        xorriso \
          -indev alpine-extended-3.23.3-x86_64.iso \
          -outdev alpine-unattended.iso \
          -map alpine.apkovl.tar.gz /alpine.apkovl.tar.gz \
          -map install.sh /install.sh \
          -map isolinux/isolinux.cfg /boot/syslinux/isolinux.cfg \
          -boot_image any replay

        rm -rf isolinux
      else
        echo "ERROR: xorriso is not installed. Please install it:"
        echo "  apt install -y xorriso"
        exit 1
      fi

      # Cleanup
      rm -rf ovl alpine.apkovl.tar.gz install.sh

      echo "=== unattended installation ISO created ==="
    else
      echo "=== unattended installation ISO already exists, skipping build ==="
    fi

    # create qcow2 image
    if [[ ! -f ~/qemu/alpine.qcow2 ]]; then
      echo "=== initializing qemu guest image ==="
      qemu-img create -f qcow2 alpine.qcow2 50G
    else
      echo "=== qcow2 image already exists, skipping creation ==="
    fi

    # install with cdrom (unattended)
    echo "=== starting unattended installation ==="
    echo ""
    echo ""
    echo ""
    echo "login with"
    echo "login: root"
    echo ""
    echo "run "
    echo "$ sh /media/cdrom/install.sh"
    echo ""
    echo ""
    sleep 10
    qemu-system-x86_64 \
      -cdrom alpine-unattended.iso -boot d \
      -smp 8 -m 4096 -vga std \
      -drive format=qcow2,file=alpine.qcow2 \
      -nic user,hostfwd=tcp:127.0.0.1:2222-:22 \
      -serial mon:stdio -nographic

    echo "=== installation complete ==="

  popd

fi

# make project dir
# mkdir -p /var/run/sshd /opt/project/uad-shizuku

# running qemu with SSH port forwarding (host:2222 -> guest:22)
if [[ "$1" == "rundebug" ]]; then
  # Check if QEMU is already running
  if check_qemu_running; then
    echo "ERROR: QEMU guest is already running!"
    echo "Check status with: ./qemu.sh status"
    exit 1
  fi

  pushd ~/qemu
    echo "=== starting QEMU guest ==="
    echo ""
    echo ""
    echo ""
    echo "login with"
    echo "login: root"
    echo "password: root"
    echo ""
    echo ""
    echo ""
    sleep 10

    qemu-system-x86_64 \
      -smp 8 -m 4096 -vga std \
      -drive format=qcow2,file=alpine.qcow2 \
      -usb -device qemu-xhci \
      -nic user,hostfwd=tcp:127.0.0.1:2222-:22 \
      -serial mon:stdio -nographic

    echo "=== QEMU guest started ==="

  popd

fi

# running qemu with SSH port forwarding (host:2222 -> guest:22)
if [[ "$1" == "run" ]]; then
  # Check if QEMU is already running
  if check_qemu_running; then
    echo "ERROR: QEMU guest is already running!"
    echo "Check status with: ./qemu.sh status"
    exit 1
  fi

  pushd ~/qemu
    echo "=== starting QEMU guest ==="

    qemu-system-x86_64 \
      -smp 8 -m 4096 -vga std \
      -drive format=qcow2,file=alpine.qcow2 \
      -usb -device qemu-xhci \
      -nic user,hostfwd=tcp:127.0.0.1:2222-:22 -display none &

    sleep 10
    echo "=== QEMU guest started ==="

  popd

fi

# stop qemu
if [[ "$1" == "stop" ]]; then
  if check_qemu_running; then
    echo "=== stopping QEMU guest ==="
    pkill -f "qemu-system-x86_64.*alpine.qcow2"
    sleep 2
    if check_qemu_running; then
      echo "=== force killing QEMU guest ==="
      pkill -9 -f "qemu-system-x86_64.*alpine.qcow2"
    fi
    echo "=== QEMU guest stopped ==="
  else
    echo "QEMU guest is not running"
  fi
fi

# get qemu status
if [[ "$1" == "status" ]]; then
  ps aux | grep qemu-system-x86_64 | grep -v grep || echo "QEMU is not running"
fi

# reset - delete qcow2 and unattended ISO
if [[ "$1" == "reset" ]]; then
  if check_qemu_running; then
    echo "ERROR: QEMU guest is already running!"
    echo "Please stop it first with: ./qemu.sh stop"
    exit 1
  fi

  echo "=== Resetting QEMU environment ==="
  echo "This will delete:"
  echo "  - ~/qemu/alpine.qcow2"
  echo "  - ~/qemu/alpine-unattended.iso"
  echo ""
  read -p "Are you sure? (y/N) " -n 1 -r
  echo
  if [[ $REPLY =~ ^[Yy]$ ]]; then
    pushd ~/qemu
      if [[ -f alpine.qcow2 ]]; then
        rm -f alpine.qcow2
        echo "  ✓ Deleted alpine.qcow2"
      else
        echo "  - alpine.qcow2 not found"
      fi

      if [[ -f alpine-unattended.iso ]]; then
        rm -f alpine-unattended.iso
        echo "  ✓ Deleted alpine-unattended.iso"
      else
        echo "  - alpine-unattended.iso not found"
      fi
    popd
    echo ""
    echo "=== Reset complete ==="
    echo "You can now run: ./qemu.sh init"
  else
    echo "Reset cancelled"
  fi
fi

# ssh mode
if [[ "$1" == "ssh" ]]; then
  ssh-keygen -f '/home/wj/.ssh/known_hosts' -R '[localhost]:2222'
  ssh -p 2222 -i ~/qemu/.qemu_ssh/id_ed25519 -o StrictHostKeyChecking=accept-new root@localhost
fi

# rsync host to guest
if [[ "$1" == "syncto" ]]; then
  rsync -avzP --exclude='.qemu' --exclude='jniLibs' --exclude='target' --exclude='build' --exclude='.solidbase' --exclude='reference' --exclude='.git' \
    --rsh="ssh -p 2222 -i ~/qemu/.qemu_ssh/id_ed25519 -o StrictHostKeyChecking=accept-new" $PROJECT_ROOT root@localhost:$CONTAINER_DIR
fi

# rsync guest to host
if [[   "$1" == "syncfrom" ]]; then
  mkdir -p $TAKEOUT_DIR
  rsync -avzP --exclude='.qemu' --exclude='target' --rsh="ssh -p 2222 -i ~/qemu/.qemu_ssh/id_ed25519 -o StrictHostKeyChecking=accept-new" \
    root@localhost:$CONTAINER_DIR/$PROJECT_NAME/$RELATIVE_DIR/$TAKEOUT_DIR/ $TAKEOUT_DIR
fi

# install debug app
if [[ "$1" == "install" ]]; then
  adb install app/build/outputs/apk/debug/app-arm64-v8a-debug.apk
fi

# get logcat per app
if [[ "$1" == "slog" ]]; then
  adb logcat --pid=$(adb shell pidof -s com.draco.uad)
fi

# get logcat
if [[ "$1" == "glog" ]]; then
  adb logcat|grep uad
fi
