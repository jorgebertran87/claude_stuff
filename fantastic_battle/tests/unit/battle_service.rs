use std::collections::HashMap;
use std::sync::Arc;

use cucumber::{given, then, when, World};

use fantastic_battle::domain::model::{
    BattleError, BattleOutcome, Player, PlayerError, Question, Theme, ThemeError,
};
use fantastic_battle::domain::ports::QuestionAsker;
use fantastic_battle::domain::service::BattleService;

#[derive(Clone, Debug)]
struct FakeQuestionAsker {
    questions: HashMap<(String, String), Question>,
}

impl QuestionAsker for FakeQuestionAsker {
    fn ask(&self, theme: &Theme, player: &Player) -> Question {
        let key = (theme.value().to_string(), player.name().to_string());
        self.questions.get(&key).cloned().unwrap()
    }
}

#[derive(Debug, Default, World)]
pub struct BattleWorld {
    theme: Option<Theme>,
    theme_result: Option<Result<Theme, ThemeError>>,
    player_result: Option<Result<Player, PlayerError>>,
    players: Vec<Player>,
    questions: HashMap<(String, String), Question>,
    current_battle: Option<fantastic_battle::domain::model::Battle>,
    asked_questions: Vec<(String, String)>,
    answer_result: Option<Result<BattleOutcome, BattleError>>,
    answer_results: Vec<Result<BattleOutcome, BattleError>>,
}

// ── Theme ────────────────────────────────────────────────────────────────────

#[when("the human player chooses the theme \"Greek mythology\"")]
fn when_choose_theme(world: &mut BattleWorld) {
    match Theme::new("Greek mythology") {
        Ok(t) => {
            world.theme = Some(t.clone());
            world.theme_result = Some(Ok(t));
        }
        Err(e) => {
            world.theme_result = Some(Err(e));
        }
    }
}

#[when("the human player chooses a blank theme")]
fn when_blank_theme(world: &mut BattleWorld) {
    let r = Theme::new("");
    world.theme_result = Some(match r {
        Ok(t) => {
            world.theme = Some(t.clone());
            Ok(t)
        }
        Err(e) => Err(e),
    });
}

#[then("the chosen theme is \"Greek mythology\"")]
fn then_theme_is(world: &mut BattleWorld) {
    let t = world.theme.as_ref().expect("theme not set");
    assert_eq!(t.value(), "Greek mythology");
}

#[then("the theme is rejected because a theme is required")]
fn then_theme_rejected(world: &mut BattleWorld) {
    let r = world.theme_result.as_ref().expect("theme result not set");
    assert_eq!(*r.as_ref().unwrap_err(), ThemeError::Required);
}

// ── AI players ───────────────────────────────────────────────────────────────

#[when("an AI player named \"Sphinx\" enters the game")]
#[given("an AI player named \"Sphinx\"")]
fn when_player_enters_sphinx(world: &mut BattleWorld) {
    let p = Player::new("Sphinx").expect("Sphinx is a valid name");
    world.players.push(p);
}

#[when("an AI player named \"Minotaur\" enters the game")]
#[given("an AI player named \"Minotaur\"")]
fn when_player_enters_minotaur(world: &mut BattleWorld) {
    let p = Player::new("Minotaur").expect("Minotaur is a valid name");
    world.players.push(p);
}

#[when("an AI player with a blank name tries to enter the game")]
fn when_blank_player(world: &mut BattleWorld) {
    let r = Player::new("");
    world.player_result = Some(r);
}

#[then("the game has an AI player named \"Sphinx\"")]
fn then_has_player_sphinx(world: &mut BattleWorld) {
    let found = world.players.iter().any(|p| p.name() == "Sphinx");
    assert!(found, "expected an AI player named Sphinx");
}

#[then("the player is rejected because a player needs a name")]
fn then_player_rejected(world: &mut BattleWorld) {
    let r = world.player_result.as_ref().expect("player result not set");
    assert_eq!(*r.as_ref().unwrap_err(), PlayerError::NameRequired);
}

// ── Question wiring ──────────────────────────────────────────────────────────

#[given(regex = r#""(.+)" will ask "(.+)" with correct answer "(.+)" for the theme "(.+)""#)]
fn given_player_question(world: &mut BattleWorld, player: String, question: String, answer: String, theme: String) {
    world.questions.insert(
        (theme, player),
        Question::new(&question, &answer),
    );
}

// ── Battling ─────────────────────────────────────────────────────────────────

#[given("the human player has chosen the theme \"Greek mythology\"")]
fn given_theme_greek(world: &mut BattleWorld) {
    when_choose_theme(world);
}

fn start_battle_against(world: &mut BattleWorld, player_name: &str) {
    let theme = world.theme.as_ref().expect("theme not set");
    let player = Player::new(player_name).expect("valid player name");
    let service = BattleService::new(Arc::new(FakeQuestionAsker {
        questions: world.questions.clone(),
    }));
    let battle = service.start_battle(theme, &player);
    world
        .asked_questions
        .push((player_name.to_string(), battle.question().text().to_string()));
    world.current_battle = Some(battle);
}

#[when(regex = r#"the human player battles "([^"]+)"(?: again)?$"#)]
#[given(regex = r#"the human player battles "([^"]+)"(?: again)?$"#)]
fn when_battle(world: &mut BattleWorld, player_name: String) {
    start_battle_against(world, &player_name);
}

#[then(regex = r#"the human player is asked "(.+)""#)]
fn then_asked(world: &mut BattleWorld, expected: String) {
    let battle = world.current_battle.as_ref().expect("no battle");
    assert_eq!(battle.question().text(), &expected);
}

#[then(regex = r#""(.+)" posed the question "(.+)""#)]
fn then_player_posed(world: &mut BattleWorld, player_name: String, question: String) {
    let found = world
        .asked_questions
        .iter()
        .any(|(p, q)| p == &player_name && q == &question);
    assert!(
        found,
        "expected {} to have posed \"{}\", but asked_questions is {:?}",
        player_name, question, world.asked_questions
    );
}

#[then(regex = r#"both battles pose the question "(.+)""#)]
fn then_both_pose(world: &mut BattleWorld, question: String) {
    assert_eq!(world.asked_questions.len(), 2);
    for (_, q) in &world.asked_questions {
        assert_eq!(q, &question);
    }
}

// ── Battle outcome ───────────────────────────────────────────────────────────

#[when(regex = r#"the human player answers "(.+)""#)]
fn when_answer(world: &mut BattleWorld, answer: String) {
    let battle = world.current_battle.as_mut().expect("no battle");
    let r = battle.answer(&answer);
    world.answer_result = Some(r);
    world.answer_results.push(r);
}

#[when("the human player tries to answer again")]
fn when_answer_again(world: &mut BattleWorld) {
    let battle = world.current_battle.as_mut().expect("no battle");
    let r = battle.answer("Zeus");
    world.answer_result = Some(r);
    world.answer_results.push(r);
}

#[then("the human player wins the battle")]
fn then_victory(world: &mut BattleWorld) {
    let r = world.answer_result.as_ref().expect("no answer result");
    assert_eq!(r.unwrap(), BattleOutcome::Victory);
}

#[then("the human player is defeated")]
fn then_defeat(world: &mut BattleWorld) {
    let r = world.answer_result.as_ref().expect("no answer result");
    assert_eq!(r.unwrap(), BattleOutcome::Defeat);
}

#[then("the answer is rejected because the battle is already over")]
fn then_answer_rejected(world: &mut BattleWorld) {
    let r = world.answer_result.as_ref().expect("no answer result");
    assert_eq!(r.unwrap_err(), BattleError::AlreadyOver);
}

#[then("the human player still wins the battle")]
fn then_still_victory(world: &mut BattleWorld) {
    let battle = world.current_battle.as_ref().expect("no battle");
    assert_eq!(battle.outcome(), Some(BattleOutcome::Victory));
}

fn main() {
    futures::executor::block_on(BattleWorld::run("features/battle_service.feature"));
}
