# Release Process for new GHEC repos

## Status
Proposed

## During Release Cutoff

Default Steps
1. Cut Eos-ripple release branch.
2. Tag all extensions and Ripple open source


If a patch is needed on an existing release
1. Create a release branch for the extension (and/or) Ripple OSS
2. Make changes and create a PR against the release branch
3. After validation merge to release branch and update the eos-ripple repo

Objective here is to not having to create new release branch for each extension for a patch intended for only some extensions

Once a new release is made create a new tag for the Extension or Ripple open source

