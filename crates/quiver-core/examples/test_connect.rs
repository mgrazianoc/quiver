use quiver_core::client::FlightClient;
use quiver_core::connection::ConnectionProfile;

#[tokio::main]
async fn main() {
    let profile = ConnectionProfile {
        name: "test".into(),
        host: "localhost".into(),
        port: 50052,
        tls_enabled: false,
        ..Default::default()
    };
    println!("Connecting to {} ...", profile.endpoint_uri());
    match FlightClient::connect(&profile).await {
        Ok(c) => println!("SUCCESS: state={:?}", c.state()),
        Err(e) => println!("FAILED: {:#}", e),
    }
}
