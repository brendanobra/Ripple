# Linchpin Client
#### A Linchpin client written in rust
Publish-related actions are currently unimplemented. If you would like to contribute, pull requests are welcome.

### Example Usage
```
let config = ClientConfig {
        url: String::from("someUrl"),
        sat: String::from("someSat"),
        initial_reconnect_delay_ms: None,
        max_reconnect_delay_ms: None,
    };

let (event_handler_tx, mut event_handler_rx) = mpsc::channel::<ClientEvent>(16);

tokio::spawn(async move {
    while let Some(event) = event_handler_rx.recv().await {
        info!("Got event": {:?}", event");
    }
});

let mut linchpin = linchpin_client::start(config, event_handler_tx).await;

if let Err(e) = linchpin.subscribe(&String::from("someTopic")).await {
    println!("Failed to subscribe: e={:?}", e);
}
```
As shown in the example above, calling:
```
pub async fn start(config: ClientConfig, client_event_handler: mpsc::Sender<ClientEvent>)
)
```
...will attempt to start a linchpin connection using the specified [configuration](#client-configuration) and [event handling](#event-handling) parameters.

### Client Configuration
```
pub struct ClientConfig {
    pub url: String,
    pub sat: String,
    pub initial_reconnect_delay_ms: Option<u64>,
    pub max_reconnect_delay_ms: Option<u64>,
}
```
#### url
The linchpin endpoint to use. The typical format is:
`wss://linchpin-ci.lp.xcal.tv:18082/listen?client=<CLIENT_ID>&deviceId=<DEVICE_ID>`. Please refer to Linchpin documentation for specifics.
#### sat
The device SAT.
#### initial_reconnect_delay_ms
Default: 1 second. If specified, the initial delay when reconnecting to linchpin after an unanticipated disconnect. If unsuccessful, subsequent reconnect attempts will be made with increasing delays.
#### max_reconnect_delay_ms
Default: 32 seconds. If specified, the maximum delay when attempting to reconnect to linchpin after an unanticipated disconnect. Starting with the initial reconnect delay, the delay will double in value with each reconnect attempt until a connection is made or the maximum delay is reached.

### Event Handling

The `client_event_handler` channel `Sender` allows the user to receive `ClientEvent` notifications upon which it can act. `ClientEvent` notifications include both connection and linchpin events:
```
pub enum ClientEvent {
    State(ClientState),
    Notify(NotifyMessage),
}
```
### ClientEvent::State(ClientState)
Indicates the current client state. If disconnected, it will include an error indicating the reason for the disconnect. If the error is `LinchpinClientError::Unauthorized`, it is likely that the SAT specified in the configuration has expired and needs renewal. See [set_sat](#set_sat) method below.
```
pub enum ClientState {
    Connecting,
    Connected,
    Reconnecting,
    Disconnected(LinchpinClientError),
}
```

### ClientEvent::Notify(NotifyMessage)
Represents a NotifyMessage as received from linchpin. The client must first [subscribe](#subscribe) to one or more topics in order to receive notifications for various topics.
```
pub struct NotifyMessage {
    pub request_id: String,
    pub headers: Option<HashMap<String, String>>,
    pub message_type: Option<String>,
    pub topic: String,
    pub payload: String,
    pub payload_type: String,
}
```
### Client Methods

#### subscribe
`pub async fn subscribe(&mut self, topic: &String) -> Result<() LinchpinClientError>`

Subscribes to the specified topic.

#### unsubscribe
`pub async fn unsubscribe(&mut self, topic: &String) -> Result<(), LinchpinClientError>`

Unsubscribes to the specified topic.

#### set_sat
`pub async fn set_sat(&self, sat: String) -> Result<(), LinchpinClientError>`

Sets the SAT. When called, the next attempt to connect will use the new value.

#### disconnect
`pub async fn disconnect(&self) -> Result<(), LinchpinClientError>`

Disconnects from linchpin.

#### reconnect
`pub async fn reconnect(&self) -> Result<(), LinchpinClientError>`

Reconnects to linchpin.