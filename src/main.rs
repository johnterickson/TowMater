use std::error::Error;

mod angle;
use angle::Angle;
use na::Vector3;
use rlbot::{RLBot, Rotator};

const BUILD_TIME : &str = include!(concat!(env!("OUT_DIR"), "/timestamp.txt"));

fn main() -> Result<(), Box<dyn Error>> {

    // for arg in std::env::args() {
    //     eprintln!("{}", arg);
    // }

    let args = rlbot::parse_framework_args()
        .map_err(|_| Box::<dyn Error>::from("could not parse framework arguments"))?
        .ok_or_else(|| Box::<dyn Error>::from("not launched by framework"))?;

    let player_index = args.player_index as usize;

    let mut bot = MyBot { 
        player_index, 
        my_goal: None, 
        opp_goal: None,
        kickoff: true,
    };

    let rlbot = rlbot::init_with_options(args.into())?;

    let mut field_info = None;

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
            Ok(input) => rlbot.update_player_input(player_index as i32, &input)?,
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
    kickoff: bool,
}

impl MyBot {
    const HEADER_1_GROUP_ID: i32 = 1000;
    const CAR_GROUP_BASE_ID: i32 = 1010;

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
        let car_vel = car.physics.velocity.to_vec3();
        let (car_pitch, car_yaw, car_roll) = {
            let angles = &car.physics.rotation;
            (Angle::from_radians(angles.pitch), Angle::from_radians(angles.yaw), Angle::from_radians(angles.roll))
        };

        let opp = &packet.players[1 - self.player_index];
        let opp_pos = opp.physics.location.to_vec3();

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

        let ball_to_my_goal = my_goal_pos - ball_pos;
        let ball_to_opp_goal = opp_goal_pos - ball_pos;
        let ball_closer_to_opp_goal = ball_to_my_goal.magnitude() > ball_to_opp_goal.magnitude();

        let car_to_opp_goal = opp_goal_pos - car_pos;
        let opp_to_opp_goal = opp_goal_pos - opp_pos;
        let breakaway = 
            ball_to_opp_goal.magnitude() < car_to_opp_goal.magnitude() &&
            car_to_opp_goal.magnitude() < opp_to_opp_goal.magnitude();

        let mut group = rlbot.begin_render_group(Self::CAR_GROUP_BASE_ID + self.player_index as i32);
        let white = group.color_rgb(255, 255, 255);
        let red = group.color_rgb(255, 0, 0);
        let green = group.color_rgb(0, 255, 0);
        let blue = group.color_rgb(0, 0, 255);
        let mag = group.color_rgb(255, 0, 255);

        
        let offense = ball_closer_to_opp_goal || breakaway;

        let (desired_ball_dir, distance_to_ball_center) = if offense {
            group.draw_string_3d((car_pos.x, car_pos.y, car_pos.z), (1,1), if breakaway { "BREAKAWAY!" } else { "OFFENSE!" }, white);
            (ball_to_opp_goal.normalize(), 1.1 * Self::BALL_RADIUS)
        } else {
            group.draw_string_3d((car_pos.x, car_pos.y, car_pos.z), (1,1), "DEFENSE!", white);
            let ball_to_my_goal_distance = ball_to_my_goal.magnitude();
            let ball_to_opp_goal_distance = ball_to_opp_goal.magnitude();
            let total_distance = ball_to_my_goal_distance + ball_to_opp_goal_distance;
            let closeness_to_my_goal = total_distance / ball_to_my_goal_distance;
            let closeness_to_opp_goal = total_distance / ball_to_opp_goal_distance;
            let avg = closeness_to_opp_goal * ball_to_opp_goal.normalize() + -1.0 * closeness_to_my_goal * ball_to_my_goal.normalize();
            (avg.normalize(), 0.9 * Self::BALL_RADIUS)
        };

        let ball_vel_dir = ball_vel.normalize().get_numeric();
        let nudge_dir = if let Some(ball_vel_dir) = ball_vel_dir {
            (desired_ball_dir - ball_vel_dir).normalize()
        } else {
            // ball_vel is 0 so it is directionless. 
            desired_ball_dir.normalize()
        };

        {
            let arrow_len = 2.0 * Self::BALL_RADIUS;
            
            let vel_arrow_tip = ball_pos + ball_vel_dir.map_or(Default::default(), |v| arrow_len * v );
            group.draw_line_3d((ball_pos.x, ball_pos.y, ball_pos.z),  (vel_arrow_tip.x, vel_arrow_tip.y, vel_arrow_tip.z), green);
            let desired_arrow_tip = ball_pos + arrow_len * desired_ball_dir;
            group.draw_line_3d((ball_pos.x, ball_pos.y, ball_pos.z),  (desired_arrow_tip.x, desired_arrow_tip.y, desired_arrow_tip.z), blue);

            let nudge_arrow_tip = vel_arrow_tip + arrow_len * nudge_dir;
            group.draw_line_3d((vel_arrow_tip.x, vel_arrow_tip.y, vel_arrow_tip.z),  (nudge_arrow_tip.x, nudge_arrow_tip.y, nudge_arrow_tip.z), mag);

        }

        let target_pos = ball_pos + -1.0 * distance_to_ball_center * nudge_dir;
        group.draw_line_3d((car_pos.x, car_pos.y, car_pos.z), (target_pos.x, target_pos.y, target_pos.z), white);
        

        // packet.game_info.
        // let (my_goal, opponent_goal) = 
        // let rlbot = rlbot::init()?;
        // let mut group = rlbot.begin_render_group(1234);
        
        let car_to_target = target_pos - car_pos;
        let car_to_ball = ball_pos - car_pos;
        let target_on_far_side_of_ball = car_to_target.magnitude() > car_to_ball.magnitude();
        let can_sneak_under_ball = car_to_target.magnitude() < ball_pos[2];
        let target_pos = if target_on_far_side_of_ball && !can_sneak_under_ball {
            group.draw_line_3d((car_pos.x, car_pos.y, car_pos.z), (target_pos.x, target_pos.y, target_pos.z), red);
            // we are on the wrong side of the ball
            let ball_to_target = car_to_target - car_to_ball;

            group.draw_line_3d((ball_pos.x, ball_pos.y, ball_pos.z), (target_pos.x, target_pos.y, target_pos.z), white);

            // split ball_to_target into the part parallel to car_to_ball and the rest
            let (_ball_to_target_parallel, ball_to_target_rest) = 
                rebasis(&car_to_ball, &ball_to_target);

            // do we already have sufficient clearance?
            if ball_to_target_rest.magnitude() > 2.0 * Self::BALL_RADIUS {
                target_pos
            } else {
                group.draw_string_3d((car_pos.x, car_pos.y, car_pos.z), (1,1), "GO AROUND!", white);
                target_pos + 1.5 * Self::BALL_RADIUS * ball_to_target_rest.normalize()
            }
        } else {
            target_pos
        };

        group.draw_line_3d((car_pos.x, car_pos.y, car_pos.z), (target_pos.x, target_pos.y, target_pos.z), white);

        let car_to_target = target_pos - car_pos;
        let desired_yaw = Angle::from_atan2(car_to_target.y, car_to_target.x);
        let steer_degrees = (desired_yaw - car_yaw).degrees();

        let (steer_degrees, throttle) = if steer_degrees.abs() < 90.0 {
            (steer_degrees, 1.0)
        } else {
            (-1.0 * steer_degrees, -1.0)
        };

        let steer_strength = 0.5;
        let steer = (steer_degrees * steer_strength).clamp(-1.0, 1.0);

        let car_to_target_xy_distance = {
            let mut car_to_target_xy = car_to_target;
            car_to_target_xy[2] = 0.0;
            car_to_target_xy.magnitude()
        };

        let mut jump = false;
        let mut car_rotate: Rotator = Default::default();

        const WALL_X: f32 = 4096.0;

        let on_wall = car_pos[2] > 2.0 * Self::BALL_RADIUS 
            && (car_pitch.degrees().abs() > 70.0 || car_roll.degrees().abs() > 70.0)
            && car.has_wheel_contact
            && car_pos.x.abs() > WALL_X - 2.0 * Self::BALL_RADIUS;

        let car_in_air = car_pos[2] > 2.0 * Self::BALL_RADIUS && !car.has_wheel_contact;
        let car_falling = car_vel[2] < 0.0;

        // if on a wall, jump off
        if on_wall {
            jump = true;
        } 
        // if we're falling through the air, then let's level off and turn in the right direction
        else if car_in_air && car_falling {
            car_rotate.roll = (-0.01 * car_roll.degrees()).clamp(-1.0, 1.0);
            car_rotate.pitch = (-0.01 * car_pitch.degrees()).clamp(-1.0, 1.0);
            car_rotate.yaw = steer;

        }
        // go for a header?
        else if car_to_target_xy_distance < Self::BALL_RADIUS { // && (car_vel - ball_vel).magnitude() < 3.0 * Self::BALL_RADIUS {
            // if above me and coming down
            if ball_pos[2] - car_pos[2] > 1.0 * Self::BALL_RADIUS  && ball_vel[2] < 0.0 {
                let ball_land_time = ball_pos[2] / (-1.0 * ball_vel[2]);
                group.draw_string_3d((car_pos.x, car_pos.y, car_pos.z + 50.0), (1,1), format!("ball_land_time: {}", ball_land_time), white);

                if ball_land_time < 0.8 {
                    jump = true;
                }
            }
        }

        let mut boost = false;

        if self.kickoff {
            if car.boost > 0 {
                boost = true;
            } else {
                self.kickoff = false;
            }
        }

        if car_to_target_xy_distance > 20.0 * Self::BALL_RADIUS && car_pos[2] < Self::BALL_RADIUS {
            boost = true;
        }
        
        let misalignment_angle = Angle::between_vecs(&ball_to_opp_goal, &car_to_ball);
        if offense && misalignment_angle.degrees().abs() < 15.0 // todo adjust for distance to goal and size of goal
        {
            boost = true;
        }

        group.render()?;

        Ok(rlbot::ControllerState {
            throttle,
            steer,
            jump,
            boost,
            pitch: car_rotate.pitch,
            roll: car_rotate.roll,
            yaw: car_rotate.yaw,
            ..Default::default()
        })
    }
}


// projection of b onto a
fn project(b: &Vector3<f32>, a: &Vector3<f32>) -> Vector3<f32> {
    let a_len = a.magnitude();
    a.dot(b) / (a_len * a_len) * a
}

fn rebasis(basis: &Vector3<f32>, v: &Vector3<f32>) -> (Vector3<f32>, Vector3<f32>) {
    let parallel = project(v, basis);
    let rest = v - parallel;
    (parallel, rest)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn project_tests() {
        let b = Vector3::new(1.0, 1.0, 0.0);
        let a = Vector3::new(1.0, 0.0, 0.0);
        let actual = project(&b, &a);
        let expected = Vector3::new(1.0, 0.0, 0.0);
        assert_approx_eq_vec(actual, expected);
    }

    #[test]
    fn rebasis_tests() {
        let basis = Vector3::new(1.0, 1.0, 0.0);
        let v = Vector3::new(1.0, 0.0, 0.0);
        let (parallel, rest) = 
        rebasis(&basis, &v);
        assert_approx_eq_vec(parallel + rest, v);
        assert_approx_eq(parallel.dot(&basis), 1.0);
    }

    fn assert_approx_eq_vec(a: Vector3<f32>, b: Vector3<f32>) {
        assert!((a - b).magnitude() < 0.01, "{} vs {}", a, b);
    }

    fn assert_approx_eq(a: f32, b: f32) {
        assert!((a - b).abs() < 0.01, "{} vs {}", a, b);
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

trait IsNumeric {
    fn get_numeric(self) -> Option<Self> where Self:Sized;
}

impl IsNumeric for Vector3<f32> {
    fn get_numeric(self) -> Option<Self> {
        for a in self.as_slice() {
            if !f32::is_finite(*a) {
                return None;
            }
        }
        Some(self)
    }
}