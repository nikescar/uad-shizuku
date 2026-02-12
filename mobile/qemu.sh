#!/usr/bin/env bash

# apt install -y qemu-system-x86 qemu-utils qemu-block-extra

# this is qemu guest management build script for running android build in qemu guest.

show_usage() {
  echo "Usage: qemu.sh <command>"
  echo ""
  echo "Commands:"
  echo "  init       - host initialization if qcow2 image not exists"
  echo "  run        - run qemu with existing qcow2 image"
  echo "  status     - check if qemu guest is running"
  echo "  ssh        - ssh into the qemu guest"
  echo "  syncto     - rsync project from host to guest"
  echo "  syncfrom   - rsync build outputs from guest to host"
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

# init mode
if [[ "$1" == "init" ]]; then
  pushd ~/qemu
    # download instller image
    if [[ ! -f ~/qemu/alpine-virt-3.23.0-x86_64.iso ]]; then
      echo "=== downloading alpine iso ==="
      wget https://dl-cdn.alpinelinux.org/alpine/v3.23/releases/x86_64/alpine-virt-3.23.0-x86_64.iso
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
https://dl-cdn.alpinelinux.org/alpine/v3.23/main
https://dl-cdn.alpinelinux.org/alpine/v3.23/community
EOF

      # generate edd25519 ssh key for root and get its public key
      mkdir -p ovl/root/.ssh .qemu_ssh
      ssh-keygen -t ed25519 -f .qemu_ssh/id_ed25519 -N '' -C 'root@alpine'
      cp .qemu_ssh/id_ed25519.pub ovl/root/.ssh/authorized_keys
      chmod 600 ovl/root/.ssh/authorized_keys
      chmod 700 ovl/root/.ssh
      
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
# Update and upgrade packages
apk update
apk upgrade

# Install useful packages
apk add bash curl wget git rsync openssh-server

# sshd config
echo "PermitRootLogin yes" >> /etc/ssh/sshd_config
echo "PasswordAuthentication yes" >> /etc/ssh/sshd_config
echo "PubkeyAuthentication yes" >> /etc/ssh/sshd_config

# Enable SSH server
rc-update add sshd default

# Optionally set a root password (uncomment if needed)
# echo "root:root" | chpasswd

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

# Copy after-install script and run it in chroot
cp /etc/auto-setup-alpine/after-install.start /mnt/etc/local.d/after-install.start
chmod +x /mnt/etc/local.d/after-install.start

# register to run local service at boot
ln -s /etc/init.d/local /mnt/etc/runlevels/default/local

rm -rf /etc/auto-setup-alpine

# Enable local service in the new system
echo "rc_before=\"local\"" >> /mnt/etc/rc.conf

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
        xorriso \
          -indev alpine-virt-3.23.0-x86_64.iso \
          -outdev alpine-unattended.iso \
          -map alpine.apkovl.tar.gz /alpine.apkovl.tar.gz \
          -boot_image any replay
      else
        echo "ERROR: xorriso is not installed. Please install it:"
        echo "  apt install -y xorriso"
        exit 1
      fi
      
      # Cleanup
      rm -rf ovl alpine.apkovl.tar.gz
      
      echo "=== unattended installation ISO created ==="
    fi

    # create qcow2 image
    if [[ ! -f ~/qemu/alpine.qcow2 ]]; then
      echo "=== initializing qemu guest image ==="
      qemu-img create -f qcow2 alpine.qcow2 50G
    fi
    
    # install with cdrom (unattended)
    echo "=== starting unattended installation ==="
    qemu-system-x86_64 \
      -cdrom alpine-unattended.iso -boot d \
      -smp 8 -m 4096 -vga std \
      -drive format=qcow2,file=alpine.qcow2 \
      -nic user,hostfwd=tcp:127.0.0.1:2222-:22 -nographic # -display none # for debug purposes
    
    echo "=== installation complete ===" 

  popd
  
fi

# make project dir
# mkdir -p /var/run/sshd /opt/project/uad-shizuku

# running qemu with SSH port forwarding (host:2222 -> guest:22)
if [[ "$1" == "run" ]]; then
  pushd ~/qemu
    echo "=== starting QEMU guest ===" 

    qemu-system-x86_64 \
      -smp 8 -m 4096 -vga std \
      -drive format=qcow2,file=alpine.qcow2 \
      -usb -device qemu-xhci \
      -nic user,hostfwd=tcp:127.0.0.1:2222-:22 -display none &
    
    echo "=== QEMU guest started ===" 

  popd

fi

# get qemu status
if [[ "$1" == "status" ]]; then
  ps aux | grep qemu-system-x86_64 | grep -v grep || echo "QEMU is not running"
fi

# ssh mode
if [[ "$1" == "ssh" ]]; then
  ssh -p 2222 -i ~/qemu/.qemu_ssh/id_ed25519 root@localhost 
fi

# rsync host to guest
if [[ "$1" == "syncto" ]]; then
  rsync -avzP --exclude='.qemu' --exclude='jniLibs' --exclude='target' --exclude='build' --exclude='.solidbase' --exclude='reference' --exclude='.git' \
    --rsh="ssh -p 2222 -i ~/qemu/.qemu_ssh/id_ed25519" $PROJECT_ROOT root@localhost:$CONTAINER_DIR
fi

# rsync guest to host
if [[ "$1" == "syncfrom" ]]; then
  mkdir -p $TAKEOUT_DIR
  rsync -avzP --exclude='.qemu' --exclude='target' --rsh="ssh -p 2222 -i ~/qemu/.qemu_ssh/id_ed25519" \
    root@localhost:$CONTAINER_DIR/$PROJECT_NAME/$RELATIVE_DIR/$TAKEOUT_DIR/ $TAKEOUT_DIR
fi
