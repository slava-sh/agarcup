use models::Point;

pub struct Command {
    pub point_: Point,
}

impl Command {
    impl_getter!(point() -> point_: Point);
    impl_setter!(set_point(point_: Point));

    pub fn new() -> Command {
        Command { point_: Point::zero() }
    }
}
