mod front;
mod backend;

fn main() {
    println!("Hello, world!");

    front::take_order();
    front::billing::bill_producer::create_bill();
    backend::cook_order();
}
