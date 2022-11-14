use std::error::Error;

mod angle;
use angle::Angle;

fn main() -> Result<(), Box<dyn Error>> {
    rlbot::run_bot(MyBot { player_index: 0 })
}

struct MyBot {
    player_index: usize,
}

impl rlbot::Bot for MyBot {
    fn set_player_index(&mut self, index: usize) {
        self.player_index = index;
    }

    fn tick(&mut self, packet: &rlbot::GameTickPacket) -> rlbot::ControllerState {
        get_input(self.player_index, packet).unwrap_or_default()
    }
}

fn get_input(
    player_index: usize,
    packet: &rlbot::GameTickPacket,
) -> Option<rlbot::ControllerState> {
    let ball = packet.ball.as_ref()?;
    let ball_loc = ball.physics.location.to_vec3();
    let car = &packet.players[player_index];
    let car_loc = car.physics.location.to_vec3();

    let car_to_ball = ball_loc - car_loc;
    let desired_yaw = Angle::from_atan2(car_to_ball.y, car_to_ball.x);
    let car_yaw = Angle::from_radians(car.physics.rotation.yaw);
    let steer_degrees = (desired_yaw - car_yaw).degrees();
    let steer_strength = 0.5;
    let steer = (steer_degrees * steer_strength).max(-1.0).min(1.0);

    Some(rlbot::ControllerState {
        throttle: 1.0,
        steer,
        ..Default::default()
    })
}

trait ToVec3 {
    fn to_vec3(&self) -> na::Vector3<f32>;
}

impl ToVec3 for rlbot::Vector3{
    fn to_vec3(&self) -> na::Vector3<f32> {
        na::Vector3::new(self.x, self.y, self.z)
    }
}
