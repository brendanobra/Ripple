# EOS Ripple


This is a workspace repo for building Ripple 2.0 OSS and its extensions

-- Ripple OSS

-- Thunder extns

-- Distributor extn

-- Observability extn

-- firebolt-devices


## Setting up the workspace

Below instructions are useful for setting up the workspace with a VS Code IDE.

### Prerequisite

1. Install VS Code
2. Install Rust Analyzer Extension in VS Code marketplace

# Ripple Opensource issue

Ripple exists in public Github.com with private visibility and it is approved for opensource but there is a window in which it will still be private. 

This inconvenience is a product of this time and will disappear once we Ripple OSS becomes public and sdk is published to crates.io
Unfortunately during this window we have to bear the difficulties of building a repo from two different Github locations.

Below instructions are needed as Ripple OSS is still private within Github.com. 

### Rust
To install rust for the 1st time on your workstation, use these instructions: https://www.rust-lang.org/tools/install

The current version of rust that Ripple compiles with is: 1.76.0

to set the default version on your workstation (which is generally a good idea to prevent issues from being hidden):

` rustup default 1.76.0`


### Setup

For security reasons its imperative that we have 3 different private/public ssh key pairs for GHEC, GHE and OSS. Suggestion below will ensure your development machine will be able to handle all 3 keys for this workspace.

1. Setup the ~/.ssh/config like below
```
Host github.comcast.com
 Hostname github.comcast.com
 AddKeysToAgent yes
 IdentityFile ~/.ssh/<private key for ghe>
 IdentitiesOnly yes
 User <ntid>

Host fireboltghec.comcast.com
 Hostname github.com
 IdentityFile ~/.ssh/<private key for firebolt ghec org>
 IdentitiesOnly yes
 User <>_comcast

Host github.com
 Hostname github.com
 IdentityFile ~/.ssh/<private key for public oss>
 IdentitiesOnly yes
 User <public oss id>

```

2. Add the below environment variables in your profile
```
export GH_OSS_NAME=<your OSS github name>
export GH_OSS_EMAIL=<your OSS github email>
```
3. Cd to the eos-ripple project folder
```
./setup
```
## Running without a device

Eos-Ripple now supports mock device extension using which you can emulate thunder responses and run Ripple without needing a device.
Command to run Ripple without a device
>./run.sh mock

# Knowledgebase 
Please refer to the [knowledge base](./kb) in the `kb` folder


# Release Mapping 

| EOS Version | Ripple Open Source Version |ripple_comcast_exension Verion | dpab_appsanity | Firebolt Devices | 
| ----------- | -------------------------- | ----------------------------- | -------------- | ---------------- |
| 1.15.0 | Ripple 1.0.0 #f744e53 | #ddf7a69 | #0df6bbb | #96f1347 | 
| 1.15.1 | Ripple 1.0.0 #b3b202b | #99e14d9 | #1a26d72 | #cde30ee |
| 1.16.0 | Ripple 1.1.0 #adac025 | #bfb2ead | #2a39af7 | #463c39c | 
| 1.16.1 | Ripple 1.1.0 #24b6f5a | #bfb2ead | #2a39af7 | #463c39c |
| 1.16.2 | Ripple 1.1.0 #f5c6e0d | #bfb2ead | #2a39af7 | #463c39c |
