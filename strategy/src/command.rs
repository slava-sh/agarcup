use models::Point;

#[derive(Debug, Clone)]
pub struct Command {
    point: Point,
    debug_messages: Vec<String>,
    #[cfg(feature = "debug")]
    debug_lines: Vec<String>,
    #[cfg(feature = "debug")]
    debug_circles: Vec<String>,
}

impl Command {
    impl_getter!(point() -> point: Point);
    impl_setter!(set_point(point: Point));
    impl_getter!(debug_messages() -> debug_messages: &Vec<String>);

    pub fn new() -> Command {
        Command {
            point: Point::zero(),
            debug_messages: vec![],
            #[cfg(feature = "debug")]
            debug_lines: vec![],
            #[cfg(feature = "debug")]
            debug_circles: vec![],
        }
    }

    pub fn from_point(point: Point) -> Command {
        let mut command = Command::new();
        command.set_point(point);
        command
    }

    pub fn add_debug_message(&mut self, message: String) {
        self.debug_messages.push(message)
    }

    #[cfg(feature = "debug")]
    pub fn add_debug_line(&mut self, message: String) {
        self.debug_messages.push(message)
    }

    #[cfg(not(feature = "debug"))]
    pub fn add_debug_line(&mut self, message: String) {}

    #[cfg(feature = "debug")]
    pub fn add_debug_circle(&mut self, message: String) {
        self.debug_messages.push(message)
    }

    #[cfg(not(feature = "debug"))]
    pub fn add_debug_circle(&mut self, message: String) {}
}
