use models::Point;

#[derive(Debug)]
pub struct Command {
    point: Point,
    debug_messages: Vec<String>,
    #[cfg(feature = "debug")]
    debug_lines: Vec<DebugLine>,
    #[cfg(feature = "debug")]
    debug_circles: Vec<DebugCircle>,
}

impl Command {
    impl_getter!(point() -> point: Point);
    impl_setter!(set_point(point: Point));
    impl_getter!(debug_messages() -> debug_messages: &Vec<String>);

    #[cfg(feature = "debug")]
    impl_getter!(debug_lines() -> debug_lines: &Vec<DebugLine>);

    #[cfg(feature = "debug")]
    impl_getter!(debug_circles() -> debug_circles: &Vec<DebugCircle>);

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
    pub fn add_debug_line(&mut self, line: DebugLine) {
        self.debug_lines.push(line)
    }

    #[cfg(not(feature = "debug"))]
    pub fn add_debug_line(&mut self, line: DebugLine) {}

    #[cfg(feature = "debug")]
    pub fn add_debug_circle(&mut self, circle: DebugCircle) {
        self.debug_circles.push(circle)
    }

    #[cfg(not(feature = "debug"))]
    pub fn add_debug_circle(&mut self, circle: DebugCircle) {}
}

#[derive(Debug)]
pub struct DebugLine {
    pub a: Point,
    pub b: Point,
    pub color: String,
    pub opacity: f64,
}

#[derive(Debug)]
pub struct DebugCircle {
    pub center: Point,
    pub radius: f64,
    pub color: String,
    pub opacity: f64,
}
