use std::{error::Error, convert::TryInto};

mod angle;
use angle::Angle;
use na::{Vector3, Vector2};
use rlbot::RLBot;

const BUILD_TIME : &str = include!(concat!(env!("OUT_DIR"), "/timestamp.txt"));

fn main() -> Result<(), Box<dyn Error>> {

    for arg in std::env::args() {
        eprintln!("{}", arg);
    }

    let mut bot = MyBot { player_index: 0, my_goal: None, opp_goal: None };

    let args = rlbot::parse_framework_args()
        .map_err(|_| Box::<dyn Error>::from("could not parse framework arguments"))?
        .ok_or_else(|| Box::<dyn Error>::from("not launched by framework"))?;

    let player_index = args.player_index;

    let rlbot = rlbot::init_with_options(args.into())?;

    let mut field_info = None;

    bot.player_index = player_index.try_into()?;

    let mut packets = rlbot.packeteer();
    loop {

        let packet = match packets.next() {
            Ok(packet) => packet,
            Err(e) => {
                eprintln!("{:?}", e);
                continue;
            },
        };

        match bot.get_input(&rlbot, field_info.as_ref(), &packet) {
            Ok(input) => rlbot.update_player_input(player_index, &input)?,
            Err(e) => eprintln!("{:?}", e),
        }

        if field_info.is_none() {
            field_info = rlbot.interface().update_field_info_flatbuffer();
        }
    }
}

struct MyBot {
    player_index: usize,
    my_goal: Option<Vector3<f32>>,
    opp_goal: Option<Vector3<f32>>,
}

impl MyBot {
    const HEADER_1_GROUP_ID: i32 = 1000;
    const CAR_GROUP_ID: i32 = 1001;

    const BALL_RADIUS: f32 = 90.0;

    fn get_input(
        &mut self,
        rlbot: &RLBot,
        field_info: Option<&rlbot::FieldInfo>,
        packet: &rlbot::GameTickPacket,
    ) -> Result<rlbot::ControllerState, Box<dyn Error>> {

        {
            let mut group = rlbot.begin_render_group(Self::HEADER_1_GROUP_ID);
            let green = group.color_rgb(0, 255, 0);
            group.draw_string_2d((10.0, 10.0), (2, 2), BUILD_TIME, green);
            group.render()?;
        }

        let field_info = field_info.ok_or("waiting for FieldInfo")?;

        let car = &packet.players[self.player_index];
        let car_pos = car.physics.location.to_vec3();

        let ball = packet.ball.as_ref().ok_or("ball not found")?;
        let ball_pos = ball.physics.location.to_vec3();
        let ball_vel = ball.physics.velocity.to_vec3();

        if self.my_goal.is_none() {
            let goals = &field_info.goals;
            for goal in goals {
                if goal.team_num == car.team {
                    self.my_goal = Some(goal.location.to_vec3());
                } else {
                    self.opp_goal = Some(goal.location.to_vec3());
                }
            }
        }

        let my_goal_pos = self.my_goal.ok_or("cannot find my goal")?;
        let opp_goal_pos = self.opp_goal.ok_or("cannot find opp goal")?;

        let distance_to_my_goal = (car_pos - my_goal_pos).magnitude();
        let distance_to_opp_goal = (car_pos - opp_goal_pos).magnitude();

        let mut group = rlbot.begin_render_group(Self::CAR_GROUP_ID);
        let white = group.color_rgb(255, 255, 255);
        let red = group.color_rgb(255, 0, 0);

        let target_pos = if distance_to_my_goal > distance_to_opp_goal {
            group.draw_string_3d((car_pos.x, car_pos.y, car_pos.z), (1,1), "OFFENSE!", white);
            let mut opp_goal_to_ball_dir = (ball_pos - opp_goal_pos).normalize();
            // exaggerate x to make it nudge more
            opp_goal_to_ball_dir[0] *= 1.5;
            let opp_goal_to_ball_dir = opp_goal_to_ball_dir.normalize();
            ball_pos + Self::BALL_RADIUS * 0.9 * opp_goal_to_ball_dir            
        } else {
            group.draw_string_3d((car_pos.x, car_pos.y, car_pos.z), (1,1), "DEFENSE!", white);
            let ball_to_my_goal_dir = (my_goal_pos - ball_pos).normalize();
            ball_pos + Self::BALL_RADIUS * 0.9 * ball_to_my_goal_dir
        };

        group.draw_line_3d((car_pos.x, car_pos.y, car_pos.z), (target_pos.x, target_pos.y, target_pos.z), white);
        

        // packet.game_info.
        // let (my_goal, opponent_goal) = 
        // let rlbot = rlbot::init()?;
        // let mut group = rlbot.begin_render_group(1234);
        
        let car_to_target = target_pos - car_pos;
        let car_to_ball = ball_pos - car_pos;
        let target_pos = if car_to_target.magnitude() > car_to_ball.magnitude() && ball_pos[2] < 2.0 * Self::BALL_RADIUS {
            group.draw_line_3d((car_pos.x, car_pos.y, car_pos.z), (target_pos.x, target_pos.y, target_pos.z), red);
            // we are on the wrong side of the ball
            let ball_to_target = car_to_target - car_to_ball;

            group.draw_line_3d((ball_pos.x, ball_pos.y, ball_pos.z), (target_pos.x, target_pos.y, target_pos.z), white);

            // a is the amount of ball_to_target parallel with car_to_ball
            let a = ball_to_target.dot(&car_to_ball) / car_to_ball.magnitude();
            let ball_to_target_parallel = a * car_to_ball.normalize();
            // B is a vector of ball_to_target that is not parallel to car_to_ball
            let ball_to_target_perpindicular = ball_to_target - ball_to_target_parallel;
            if ball_to_target_perpindicular.magnitude() > 2.0 * Self::BALL_RADIUS {
                target_pos
            } else {
                group.draw_string_3d((car_pos.x, car_pos.y, car_pos.z), (1,1), "GO AROUND!", white);
                target_pos + 2.0 * Self::BALL_RADIUS * ball_to_target_perpindicular.normalize()
            }
        } else {
            target_pos
        };

        group.draw_line_3d((car_pos.x, car_pos.y, car_pos.z), (target_pos.x, target_pos.y, target_pos.z), white);



        let car_to_target = target_pos - car_pos;
        let desired_yaw = Angle::from_atan2(car_to_target.y, car_to_target.x);
        let car_yaw = Angle::from_radians(car.physics.rotation.yaw);
        let steer_degrees = (desired_yaw - car_yaw).degrees();

        let (steer_degrees, throttle) = if steer_degrees.abs() < 90.0 {
            (steer_degrees, 1.0)
        } else {
            (-1.0 * steer_degrees, -1.0)
        };

        let steer_strength = 0.5;
        let steer = (steer_degrees * steer_strength).max(-1.0).min(1.0);

        let car_to_target_xy_distance = {
            let mut car_to_target_xy = car_to_target;
            car_to_target_xy[2] = 0.0;
            car_to_target_xy.magnitude()
        };

        let mut jump = false;

        if car_to_target_xy_distance < Self::BALL_RADIUS {
            // if above me and coming down
            if ball_pos[2] - car_pos[2] > 1.0 * Self::BALL_RADIUS  && ball_vel[2] < 0.0 {
                let ball_land_time = ball_pos[2] / (-1.0 * ball_vel[2]);
                group.draw_string_3d((car_pos.x, car_pos.y, car_pos.z + 50.0), (1,1), format!("ball_land_time: {}", ball_land_time), white);

                if ball_land_time < 0.5 {
                    jump = true;
                }
            }
        }

        group.render()?;

        Ok(rlbot::ControllerState {
            throttle,
            steer,
            jump,
            ..Default::default()
        })
    }
}

trait ToVec3 {
    fn to_vec3(&self) -> na::Vector3<f32>;
}

impl ToVec3 for rlbot::Vector3 {
    fn to_vec3(&self) -> na::Vector3<f32> {
        na::Vector3::new(self.x, self.y, self.z)
    }
}


impl ToVec3 for rlbot::flat::Vector3 {
    fn to_vec3(&self) -> na::Vector3<f32> {
        na::Vector3::new(self.x(), self.y(), self.z())
    }
}