## Android

### Requirements

* rust 1.81
* android ndk, sdk, java, llvm, kotlin, and gradle will install with build.sh

### Building
```bash
export ANDROID_NDK_HOME="path/to/ndk"
export ANDROID_HOME="path/to/sdk"

rustup target add aarch64-linux-android
cargo install cargo-ndk

cargo ndk -t arm64-v8a -o app/src/main/jniLibs/ build --release
gradle build
gradle installDebug
```

### Logcat

```bash
# clar logcat
$ adb logcat -c
# full logcat
$ adb logcat -v time -s *:V > fullcat.log
# app specific logcat
$ adb logcat -s UAD-Shizuku > uadcat.log
```

### Create Upload Keystore

generate keystore file in ./mobile/app dir
```bash
$ ~/.local/jdk-24.0.1/bin/keytool -genkey -v -keystore release.keystore -keyalg RSA -keysize 2048 -validity 10000 -alias upload
```

check key password
```bash
$ ~/.local/jdk-24.0.1/bin/keytool -list -v -keystore release.keystore -storepass <STOREPASSWORD> -alias upload -keypass <STOREPASSWORD>
```

create keystore.properties file in ./mobile/app dir
```bash
storePassword=<STOREPASSWORD>
keyPassword=<STOREPASSWORD>
keyAlias=upload
storeFile=release.keystore
```

### Android keystore for github workflow 

Set github secrets on -Repository *Settings > -Security > -Secrets and variables > *Actions > -Repository secrets.

Export keystore to github vars.
```bash
$ base64 release.keystore > release.keystore.base64
# KEYSTORE_BASE64=<ENCODED_KEY>
# STORE_PASSWORD=<STOREPASSWORD>
# KEY_PASSWORD=<STOREPASSWORD>
# KEY_ALIAS=upload
```

### Upload signing key on google play

To upload java singing keystore to google play. You need download upload-encryption key from store.

App Integrity > Change Signing key > Export and upload a key(not using Java Keystore) > Download encryption public key
move it to ```./android/app/```.

```bash
$ cd android/app
$ wget https://www.gstatic.com/play-apps-publisher-rapid/signing-tool/prod/pepk.jar
$ java -jar pepk.jar --keystore=release.keystore --alias=upload --output=release-signing-play-generated.zip --include-cert --rsa-aes-encryption --encryption-key-path=encryption_public_key.pem
```
upload created ```release-signing-play-generated.zip``` file.

### Submit app to Google Playstore
1. register google play console
2. internal testing and pass 14 days with published testing
3. add credentials to github repository secrets
4. run github workflow

### App testing
* Closed Testing Service Provider
https://www.testerscommunity.com/app-details/LOZEF4vnbuI4DQdaRfs1
* Google Play Console Pulishing Overview
https://play.google.com/console/u/1/developers/8469971848379081167/app/4976079442462544107/publishing
```
- android test link : https://play.google.com/store/apps/details?id=pe.nikescar.uad_shizuku
- additional test invitation link : https://play.google.com/apps/internaltest/4700175684927727957
- join on the web link : https://play.google.com/apps/testing/pe.nikescar.uad_shizuku
- youtube instructions : https://www.youtube.com/shorts/OuPw-hi4-c4
- shizuku app link : https://play.google.com/store/apps/details?id=moe.shizuku.privileged.api&hl=en
- other instructions : https://uad-shizuku.pages.dev/docs/usage
```

### Submit app to Fdroid
1. fork fdroid data repository in gitlab (https://gitlab.com/fdroid/fdroiddata)
2. clone forked repository (git clone https://gitlab.com/nikescar/fdroiddata)
3. make branch with new package name (git checkout -b pe.nikescar.uad_shizuku)
4. add fdoid metadata (../fastlane/fdroid__pe.nikescar.uad_shizuku.yml)
5. commit and push it to gitlab (git commit -a -m 'initial commit' && git push origin)
6. make PR to check build pipeline (make sure PR Contents uses fdroid templates)

docs : https://f-droid.org/en/docs/

### Publish to Flathub
https://docs.flathub.org/docs/for-app-authors/submission

### Publish to Huawei
https://docs.nhncloud.com/en/Mobile%20Service/IAP/en/console-huawei-guide/

