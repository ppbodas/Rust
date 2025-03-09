use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    println!("Hello, world!");

    println!("Case for Rc<RefCell<i32>>");

    let data = Rc::new(RefCell::new(5));

    let owner1 = data.clone();
    let owner2 = data.clone();

    increment_data(&owner1);
    increment_data(&owner2);


    println!("{}", data.borrow());

    println!("Case for RefCell<Rc<i32>>");

    let rc1 = Rc::new(3);
    let rc2 = Rc::new(10);

    let cell = RefCell::new(rc1.clone());
    println!("{}", cell.borrow());

    cell.replace(rc2.clone());
    println!("{}", cell.borrow());
}

fn increment_data(data_owner: &Rc<RefCell<i32>>) {
    *data_owner.borrow_mut() += 1;
}
