# Ripple XVP Extension
Repository which contains the implementation for XVP Ripple Extension.

### Setup for Ripple Workspace
Ripple Extensions are generally used inside a ripple workspace. Follow the Ripple Workspace setup instruction if you are working on this extension within a workspace.

### Setup for Standalone
If running as standalone follow the below steps

1. `cd <worksspace directory>`
2. `git clone git@github.comcast.com:ottx/ripple_sdk.git`
3. `git clone git@github.comcast.com:ottx/ripple_extn_xvp.git`
4. `cd ripple_extn_xvp`
5. `cargo test`

#### Ripple SDK Dependency
Ripple Extensions can also run outside ripple workspaces. As Ripple SDK is still in the process of
deployed as a crate. Expectation is for the ripple_sdk library to co exist with the extension folder.
Below is a good example 

```
---<Workspace Folder>
   | - ripple_sdk
   | - ripple_extn_xvp
```

### Running extension tests locally
