# Native Build/Cross Compiling for RDK devices
This document applies almost entirely to Ripple 2.0, but similar steps could apply to Ripple 1.0

The native build (also known as "build for the device" or "building for the SOC (System On a Chip)" or "cross compiling") is accomplished via a docker image. A docker image is used 
because the build is not a standalone cross compile: It requires using the RDK toolchain/sysroot for the particular Yocto version ripple is being built for.

# Prerequisites (for everyone)
- a working install of Docker (installation left as an exercise to the reader, steps depend on platform)
- Ripple source code that builds on host (aka "your laptop")

# Prerequisites (for Comcast employees)
- aadawscli (https://github.com/cloud-cre/AWSAzureADCLI/releases)
- aws client (https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)
- Account access to the OTT Platform AWS Account (talk to @bobra200 or @kpears201 for access if you do not have it)


# Notes
Ripple is currently supported on *both* "Dunfell" and "Kirkstone" Yocto versions. This means that the correct
build docker image must be selected based on what Yocto version is being targeted.
Dunfell is the current targeted version.


## Comcast employees 
login to ECR:

```
aadawscli --account-id 318438517054
aws ecr get-login-password --region us-east-1 --profile saml | docker login --username AWS --password-stdin 318438517054.dkr.ecr.us-east-1.amazonaws.com

```
## Sky employees:
Due to lack of access to Comcast' AWS account(s), the docker image must be downloaded directly. Contact @bobra200 for access to a download, and then 


1) `gzip -d <ripple-build-tools.tar.gz>`
2) `docker load ripple-build-tools.tar`
 2a) delete local copy tarfile, not needed anymore

# Building (once image is loaded, for both Comcast and Sky)
```
cd eos-ripple
docker run -it -v `pwd`:/ripple  318438517054.dkr.ecr.us-east-1.amazonaws.com/ripple/build-tools:rust-1.69.0-dunfell
```
which should end up at something similar to :
```
root@726dcc8a08c2:/# 
```
To build a native binary, use the `/native-build.sh` wrapper script , passing it correct args , for instance:
```
RIPPLE_BUILD_ARGS="--release" RIPPLE_FEATURES="sysd"  RIPPLE_DIR=/ripple/Ripple /native-build.sh 
```

# Artifacts (aka "where are my built , native binaries?")
Because this approach uses a mapped directory on your workstation, the files will be where the
"normally" are in the `target` folder, but under the `armv7-unknown-linux-gnueabihf` directory
For instance, for a `--release` build, built artifacts will be in

`eos-ripple/Ripple/target/armv7-unknown-linux-gnueabihf/release/`





