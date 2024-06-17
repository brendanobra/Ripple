# Project Structure
eos-ripple is an "aggregator" project that facilitates one integration point for 
- Ripple - the open source components
- ripple-bolt-extn - Automation Extension
- ripple-eos-thunder-extn - Thunder communcation extension
- ripple-eos-observability - Observability metrics 
- ripple-eos-distributor-extn - proprietary commucation extension
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
|-- ripple-bolt-extn <- submodule
|-- ripple-eos-thunder-extn <- submodule
|-- ripple-eos-observability <- submodule
|-- ripple-eos-distributor-extn <- submodule

```
