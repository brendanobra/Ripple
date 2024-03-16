# Partner Artifactory 

The general information about Cargo registries is available [here](https://github.comcast.com/ottx/eos-ripple/blob/RPPL-1468/.cargo/config.toml). 

Partner artifactory is an alternate cargo registry where we can store rust crates built with closed source.  Unlike crates.io, Partner artifactory is a private registry. 

eos-ripple repository (package type Cargo) was created via RDK service request RDKS REQ-35550  

## JFrog Artifactory 
JFrog documentation is [here](https://jfrog.com/help/r/jfrog-artifactory-documentation/cargo-package-registry).

https://partners.artifactory.comcast.com/ui/repos/tree/General/eos-ripple


JFrog uses SSO, once you logged in click on < Set Me Up > and follow the instructions in < Generate Token and Crate instructions > 

 

## Artifactory Server Configuration 
https://partners.artifactory.comcast.com/artifactory/eos-ripple/config.json 
```
{ 
  "dl" : "https://partners.artifactory.comcast.com/artifactory/api/cargo/eos-ripple/v1/crates", 
  "api" : "https://partners.artifactory.comcast.com/artifactory/api/cargo/eos-ripple" 
} 
```

## Artifactory Local Configuration  

Necesary changes for the local configuration are in
https://github.comcast.com/ottx/eos-ripple/blob/main/.cargo/config.toml &
https://github.comcast.com/ottx/dpab_appsanity/blob/ripple-2-main/.cargo/config.toml

## Authentication 
Currently partner_artifactory is setup to be annonymous, if that change then the following is necessary. 

“cargo login –registry partner_artifactory” will prompt you for the token and save the credentials in ~/.cargo/credentials.toml OR update it directly as follows 

 
~/.cargo/credentials.toml: 
```
[registries.partner_artifactory] 
token = "Bearer <TOKEN>" 
```
 

## Publishing crates 
WARNING: Do not publish without updating the version.

To publish ottx-protos-create follow in the instructions in README.md
> Source: https://github.comcast.com/ottx/ottx-protos-crate 

 
To publish distro-protos-crate follow in the instructions in README.md 
> Source: https://github.comcast.com/ottx/distro-protos-crate 

 
Once published crate cannot be deleted, to prevent using a specific version, use yank 

> cargo yank [options] --version version [crate] 

Index of eos-ripple/crates: https://partners.artifactory.comcast.com/ui/native/eos-ripple/ 

 

## Using the crates 

To use the create from different registry, simply specify the registry name along with the crate information in Cargo.toml 

 
Following 2 lines were added to Cargo.toml 
```
distro_protos = { version = "0.1.5", registry="partner_artifactory" } 
ottx_protos = { version = "0.1.0", registry="partner_artifactory" } 
```
 
