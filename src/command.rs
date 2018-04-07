use models::{Point, HasPoint};

#[derive(Debug, Default)]
pub struct Command {
    point_: Point,
    split_: bool,
    pause_: bool,
    debug_messages_: Vec<String>,
    #[cfg(feature = "debug")]
    debug_lines_: Vec<DebugLine>,
    #[cfg(feature = "debug")]
    debug_circles_: Vec<DebugCircle>,
}

impl HasPoint for Command {
    fn point(&self) -> Point {
        self.point_
    }
}

impl Command {
    pub fn new() -> Command {
        Default::default()
    }

    pub fn from_point(point: Point) -> Command {
        let mut command = Command::new();
        command.set_point(point);
        command
    }

    pub fn set_point(&mut self, point: Point) {
        self.point_ = point;
    }

    pub fn split(&self) -> bool {
        self.split_
    }

    pub fn set_split(&mut self) {
        self.split_ = true;
    }

    pub fn pause(&self) -> bool {
        self.pause_
    }

    pub fn set_pause(&mut self) {
        self.pause_ = true;
    }

    pub fn debug_messages(&self) -> &[String] {
        self.debug_messages_.as_ref()
    }

    #[allow(dead_code)]
    pub fn add_debug_message(&mut self, message: String) {
        self.debug_messages_.push(message)
    }

    #[cfg(feature = "debug")]
    pub fn debug_lines(&self) -> &[DebugLine] {
        self.debug_lines_.as_ref()
    }

    #[cfg(feature = "debug")]
    pub fn add_debug_line(&mut self, line: DebugLine) {
        self.debug_lines_.push(line)
    }

    #[cfg(feature = "debug")]
    pub fn debug_circles(&self) -> &[DebugCircle] {
        self.debug_circles_.as_ref()
    }

    #[cfg(feature = "debug")]
    pub fn add_debug_circle(&mut self, circle: DebugCircle) {
        self.debug_circles_.push(circle)
    }
}

#[cfg(feature = "debug")]
#[derive(Debug)]
pub struct DebugLine {
    pub a: Point,
    pub b: Point,
    pub color: String,
    pub opacity: f64,
}

#[cfg(feature = "debug")]
#[derive(Debug)]
pub struct DebugCircle {
    pub center: Point,
    pub radius: f64,
    pub color: String,
    pub opacity: f64,
}
