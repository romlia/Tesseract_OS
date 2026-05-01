use libseat::{Seat, SeatListener};
use std::rc::Rc;
use std::cell::RefCell;

struct MyListener {
    enabled: Rc<RefCell<bool>>,
}

impl SeatListener for MyListener {
    fn enable_seat(&mut self, seat: &mut Seat) {
        println!("Seat enabled!");
        *self.enabled.borrow_mut() = true;
    }
    fn disable_seat(&mut self, seat: &mut Seat) {
        println!("Seat disabled!");
        *self.enabled.borrow_mut() = false;
        seat.disable_seat();
    }
}

fn main() {
    let enabled = Rc::new(RefCell::new(false));
    let mut listener = MyListener { enabled: enabled.clone() };
    
    let mut seat = match Seat::open(&mut listener) {
        Ok(s) => s,
        Err(e) => {
            println!("Failed to open seat: {:?}", e);
            return;
        }
    };

    println!("Seat opened: {}", seat.name());
    
    while !*enabled.borrow() {
        seat.dispatch(-1).unwrap();
    }
    
    println!("Requesting device /dev/dri/card1...");
    match seat.open_device("/dev/dri/card1") {
        Ok((id, fd)) => {
            println!("Got device fd: {}", fd);
            seat.close_device(id).unwrap();
        }
        Err(e) => {
            println!("Failed to open device: {:?}", e);
        }
    }
}
