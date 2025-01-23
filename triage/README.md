### Journeys, aka "tracing"
This is tooling to consume (as a human) `LogSignal`s emitted by Ripple. This is a step towards easier triage.

The user tools are currently command line only (and will hopefully evolve soon).
To use the tools
```
cd triage
./journeys.sh list /tmp/ripple.log

[
  {
    "session_id": "459b3005-c821-4170-aee2-b328d0dfe5a6",
    "request_id": "730534a5-9ea3-4425-bbc4-e16dcdc0ee9a",
    "method": "localization.language"
  },
  {
    "session_id": "e8d29937-a876-4522-827a-306cec80a539",
    "request_id": "aa36aa5b-78d8-40e7-b66c-bc7bae6e644a",
    "method": "device.name"
  },
  {
    "session_id": "b6d2d1e1-908c-4dc3-90f5-c6cd5f554773",
    "request_id": "7cfa96ee-6f77-44e6-aeac-e6f151e9c68c",
    "method": "localization.countryCode"
  },
  {
    "session_id": "599067b1-573c-4063-a1a9-1c023b180191",
    "request_id": "599632b2-5762-4214-8f0b-e358a79ae15d",
    "method": "device.name"
  },
  {
    "session_id": "84cfeb07-fa4a-4938-8c31-743b36cd90d1",
    "request_id": "daea78eb-438e-4e25-bdc2-9c3de5d9be40",
    "method": "device.audio"
  },
  {
    "session_id": "94de5af2-9daf-47d4-91c9-70b4e5816a31",
    "request_id": "43bcd324-b25a-4e97-bd01-ee51f397cbcb",
    "method": "device.network"
  }
]
```
pick a request id, and show it's journey:
```
./journeys.sh show /tmp/ripple.log 43bcd324-b25a-4e97-bd01-ee51f397cbcb

[
  {
    "log_signal": {
      "call_context": {
        "app_id": "refui",
        "call_id": 1,
        "cid": "cb18f7e6-536c-4443-8428-8c6f1f3bf875",
        "gateway_secure": false,
        "method": "device.network",
        "request_id": "43bcd324-b25a-4e97-bd01-ee51f397cbcb",
        "session_id": "94de5af2-9daf-47d4-91c9-70b4e5816a31"
      },
      "diagnostic_context": {},
      "message": "starting brokerage",
      "name": "handle_brokerage"
    }
  },
  {
    "log_signal": {
      "call_context": {
        "app_id": "refui",
        "call_id": 1,
        "cid": "cb18f7e6-536c-4443-8428-8c6f1f3bf875",
        "gateway_secure": false,
        "method": "device.network",
        "request_id": "43bcd324-b25a-4e97-bd01-ee51f397cbcb",
        "session_id": "94de5af2-9daf-47d4-91c9-70b4e5816a31"
      },
      "diagnostic_context": {
        "rule_alias": "org.rdk.Network.getInterfaces",
        "static": "org.rdk.Network.getInterfaces"
      },
      "message": "rule found",
      "name": "handle_brokerage"
    }
  },
  {
    "log_signal": {
      "call_context": {
        "app_id": "refui",
        "call_id": 1,
        "cid": "cb18f7e6-536c-4443-8428-8c6f1f3bf875",
        "gateway_secure": false,
        "method": "device.network",
        "request_id": "43bcd324-b25a-4e97-bd01-ee51f397cbcb",
        "session_id": "94de5af2-9daf-47d4-91c9-70b4e5816a31"
      },
      "diagnostic_context": {
        "handled": "true"
      },
      "message": "brokerage complete",
      "name": "handle_brokerage"
    }
  },
  {
    "log_signal": {
      "call_context": {
        "app_id": "refui",
        "call_id": 9,
        "cid": "cb18f7e6-536c-4443-8428-8c6f1f3bf875",
        "gateway_secure": false,
        "method": "device.network",
        "request_id": "43bcd324-b25a-4e97-bd01-ee51f397cbcb",
        "session_id": "94de5af2-9daf-47d4-91c9-70b4e5816a31"
      },
      "diagnostic_context": {
        "updated_request": "[\"{\\\"id\\\":9,\\\"jsonrpc\\\":\\\"2.0\\\",\\\"method\\\":\\\"org.rdk.Network.getInterfaces\\\",\\\"params\\\":{}}\"]"
      },
      "message": "sending message to thunder",
      "name": "thunder_broker"
    }
  },
  {
    "log_signal": {
      "call_context": {
        "app_id": "refui",
        "call_id": 9,
        "cid": "cb18f7e6-536c-4443-8428-8c6f1f3bf875",
        "gateway_secure": false,
        "method": "device.network",
        "request_id": "43bcd324-b25a-4e97-bd01-ee51f397cbcb",
        "session_id": "94de5af2-9daf-47d4-91c9-70b4e5816a31"
      },
      "diagnostic_context": {
        "response": "BrokerOutput { data: JsonRpcApiResponse { jsonrpc: \"2.0\", id: Some(9), result: Some(Object {\"interfaces\": Array [Object {\"connected\": Bool(true), \"enabled\": Bool(true), \"interface\": String(\"ETHERNET\"), \"macAddress\": String(\"f0:46:3b:5b:fa:53\")}, Object {\"connected\": Bool(false), \"enabled\": Bool(true), \"interface\": String(\"WIFI\"), \"macAddress\": String(\"f0:46:3b:5b:fa:54\")}], \"success\": Bool(true)}), error: None, method: None, params: None } }"
      },
      "message": "received message from thunder",
      "name": "thunder_response"
    }
  },
  {
    "log_signal": {
      "call_context": {
        "app_id": "refui",
        "call_id": 1,
        "gateway_secure": false,
        "method": "device.network",
        "request_id": "43bcd324-b25a-4e97-bd01-ee51f397cbcb",
        "session_id": "94de5af2-9daf-47d4-91c9-70b4e5816a31"
      },
      "diagnostic_context": {},
      "message": "broker request found",
      "name": "start_forwarder"
    }
  },
  {
    "log_signal": {
      "call_context": {
        "app_id": "refui",
        "call_id": 1,
        "gateway_secure": false,
        "method": "device.network",
        "request_id": "43bcd324-b25a-4e97-bd01-ee51f397cbcb",
        "session_id": "94de5af2-9daf-47d4-91c9-70b4e5816a31"
      },
      "diagnostic_context": {},
      "message": "processing event",
      "name": "start_forwarder"
    }
  }
]
```
