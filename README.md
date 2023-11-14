# EOS Ripple


This is a workspace repo for building Ripple 2.0 OSS and its extensions

-- Ripple OSS

-- Comcast extns

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

### Setup


1. Clone EOS Ripple Repo
>git clone git@github.comcast.com:ottx/eos-ripple.git

2. > cd eos-ripple

3. Open it in VSCode
> code .

3. Open Terminal 
> ./setup

4. Setup opensource Ripple with correct github creds. Skip this step if you already signed RDK consent with your comcast Github id
> cd Ripple

> git config user.name "Your public name"

> git config user.email "yourpublicemail@github.com"

# Knowledgebase 
Please refer to the [knowledge base](./kb) in the `kb` folder
