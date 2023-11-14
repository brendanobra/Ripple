# Project Structure
eos-ripple is an "aggregator" project that facilitates one integration point for 
- Ripple - the open source components
- ripple_comcast_extensions - proprietary extensions to ripple
- firebolt-devices - device manifest and configurations that are used to configure ripple at runtime.


# Layout

```
eos-ripple
|-- docs
|   `-- puml
|-- firebolt-devices <- submodule
|   |
|-- kb
|   |-- build
|   |-- code
|   `-- contribution
|-- Ripple <- submodule
|   |-- core
|   |-- device
|   |-- distributor
|   |-- docs
|   |-- examples
|   `-- systemd
`-- ripple_comcast_extns <- submodule
    |-- badger 
    |-- dpab_appsanity <- submodule
    |-- dpab_core <- submodule

```

