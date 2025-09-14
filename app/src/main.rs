use domain::core::BrokerX;

fn main() {
    let mut broker_x = BrokerX::new();
    broker_x.debug_populate();
    println!("{broker_x:#?}");
}
