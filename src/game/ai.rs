/// Enemy AI state machine — corresponds to WL_STATE.C.
///
/// Each actor has a `mode` (Stand / Path / Chase / Shoot / Pain / Die).
/// On each tick, `think()` selects the appropriate handler and advances the
/// animation frame counter.
///
/// Key original functions ported here:
///   T_Stand, T_Path, T_Chase, T_Shoot, T_BlowUp, A_Die
use crate::game::actor::{Actor, ActorMode, Dir};
use crate::game::player::Player;
use crate::math::Fixed;

/// Master think function — called once per actor per frame.
pub fn think(actor: &mut Actor, player: &Player) {
    actor.tic_count -= 1;

    match actor.mode {
        ActorMode::Stand => t_stand(actor, player),
        ActorMode::Path => t_path(actor, player),
        ActorMode::Chase => t_chase(actor, player),
        ActorMode::Shoot => t_shoot(actor, player),
        ActorMode::Pain => t_pain(actor),
        ActorMode::Die => t_die(actor),
        ActorMode::Dead | ActorMode::Static => {}
    }
}

/// Actor is standing still, checking line of sight periodically.
fn t_stand(actor: &mut Actor, player: &Player) {
    if actor.tic_count > 0 {
        return;
    }
    actor.tic_count = 10;

    // if the standing actor detects the player, switch mode to chase
    if sight_player(actor, player) {
        actor.mode = ActorMode::Chase;
    }
}

/// Actor is patrolling along a pre-set path.
fn t_path(actor: &mut Actor, player: &Player) {
    // if the patrolling actor detects the player, swithc mode to chase
    if sight_player(actor, player) {
        actor.mode = ActorMode::Chase;
        return;
    }

    // TODO: advance along patrol path
}

/// Actor is chasing the player using simple direct movement.
fn t_chase(actor: &mut Actor, player: &Player) {
    // if the chasing actor loses sight of the player, switch mode to patrolling
    if !sight_player(actor, player) {
        // Lost sight — go back to patrolling
        actor.mode = ActorMode::Path;
        return;
    }

    // Move toward player
    let dx = player.x - actor.x;
    let dy = player.y - actor.y;
    let dist_sq = dx * dx + dy * dy;

    // Attack threshold (~1.5 tiles)
    let attack_range = Fixed::from_f32(1.5 * 1.5);
    if dist_sq < attack_range {
        actor.mode = ActorMode::Shoot;
        actor.tic_count = 15;
        return;
    }

    // Normalize and move
    if dist_sq != Fixed::ZERO {
        let len = approximate_dist(dx, dy);
        if len != Fixed::ZERO {
            actor.x = actor.x + dx * actor.speed / len;
            actor.y = actor.y + dy * actor.speed / len;
        }
    }
}

/// Actor is in an attack animation.
fn t_shoot(actor: &mut Actor, player: &Player) {
    if actor.tic_count > 0 {
        return;
    }
    // shoot, then go back to chasing
    // TODO: fire projectile / hitscan
    actor.mode = ActorMode::Chase;
}

/// Actor is reacting to being hit.
fn t_pain(actor: &mut Actor) {
    if actor.tic_count > 0 {
        return;
    }
    // wince, then chase :) 
    actor.mode = ActorMode::Chase;
}

/// Actor is playing a death animation.
fn t_die(actor: &mut Actor) {
    if actor.tic_count > 0 {
        return;
    }
    // the chase is over. RIP
    actor.mode = ActorMode::Dead;
}

/// Quick line-of-sight check (no occlusion yet).
fn sight_player(actor: &Actor, player: &Player) -> bool {
    let dx = player.x - actor.x;
    let dy = player.y - actor.y;
    let dist_sq = (dx * dx + dy * dy).to_f32();
    dist_sq < 10.0 * 10.0 // within 10 tiles
}

/// Fast approximate distance (octagon approximation from the original).
fn approximate_dist(dx: Fixed, dy: Fixed) -> Fixed {
    let ax = dx.abs();
    let ay = dy.abs();
    if ax > ay {
        ax + ay * Fixed::from_f32(0.5)
    } else {
        ay + ax * Fixed::from_f32(0.5)
    }
}
