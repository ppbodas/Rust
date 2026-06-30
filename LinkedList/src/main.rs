fn main() {
    println!("Hello, world!");
    let mut head = None;
    add_node(&mut head, 1);
    add_node(&mut head, 2);
    add_node(&mut head, 3);
    add_node(&mut head, 4);
    add_node(&mut head, 5);

    // Print the linked list
    let mut current = &head;
    while current.is_some() {
        println!("{}", current.as_ref().unwrap().val);
        current = &current.as_ref().unwrap().next;
    }
}

struct ListNode {
    val: i32,
    next: Option<Box<ListNode>>,
}

impl ListNode {
    fn new(val: i32) -> Self {
        ListNode { val, next: None }
    }
}

fn add_node(head: &mut Option<Box<ListNode>>, val: i32) {

    let new_node = Some(Box::new(ListNode::new(val)));
    match head {
        None => *head = new_node,
        Some(node) => {
            let mut current = node;
            while let Some(ref mut next) = current.next {
                current = next;
            }
            current.next = new_node;
        }
    }
}
