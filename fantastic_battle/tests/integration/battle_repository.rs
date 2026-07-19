use cucumber::{given, then, when, World};

use fantastic_battle::container;
use fantastic_battle::domain::model::{Battle, BattleOutcome, Player, Question};
use fantastic_battle::domain::ports::BattleRepository;

#[derive(Debug, Default, World)]
pub struct BattleRepoWorld {
    repo: Option<Box<dyn BattleRepository>>,
    session_id: Option<String>,
    battle: Option<Battle>,
    found_battle: Option<Battle>,
    outcome: Option<BattleOutcome>,
}

impl BattleRepoWorld {
    fn ensure_repo(&mut self) {
        if self.repo.is_none() {
            self.repo = Some(container::test_battle_repository());
        }
    }
}

#[given(regex = r#"^a battle for session "([^"]*)" with question "([^"]*)"$"#)]
fn given_battle(world: &mut BattleRepoWorld, session_id: String, question_text: String) {
    let opponent = Player::new("Sphinx").unwrap();
    let question = Question::new(&question_text, "Zeus");
    world.session_id = Some(session_id);
    world.battle = Some(Battle::new(opponent, question));
}

#[given(regex = r#"^a battle for session "([^"]*)" with question "([^"]*)" and answer "([^"]*)"$"#)]
fn given_battle_with_answer(
    world: &mut BattleRepoWorld,
    session_id: String,
    question_text: String,
    correct_answer: String,
) {
    let opponent = Player::new("Sphinx").unwrap();
    let question = Question::new(&question_text, &correct_answer);
    world.session_id = Some(session_id);
    world.battle = Some(Battle::new(opponent, question));
}

#[when("the battle is saved")]
fn when_save_battle(world: &mut BattleRepoWorld) {
    world.ensure_repo();
    let repo = world.repo.as_ref().unwrap();
    let battle = world.battle.take().expect("no battle to save");
    let session_id = world.session_id.as_ref().expect("no session id");
    repo.save(session_id, battle);
}

#[when(regex = r#"^the human player answers "([^"]*)"$"#)]
fn when_answer(world: &mut BattleRepoWorld, answer: String) {
    world.ensure_repo();
    let repo = world.repo.as_ref().unwrap();
    let session_id = world.session_id.as_ref().expect("no session id");
    let mut battle = repo.find(session_id).expect("battle not found");
    world.outcome = Some(battle.answer(&answer).unwrap());
    repo.save(session_id, battle);
}

#[then(regex = r#"^the battle can be found by session id "([^"]*)"$"#)]
fn then_find_battle(world: &mut BattleRepoWorld, session_id: String) {
    world.ensure_repo();
    let repo = world.repo.as_ref().unwrap();
    world.found_battle = repo.find(&session_id);
}

#[then(regex = r#"^the found battle has question "([^"]*)"$"#)]
fn then_found_battle_question(world: &mut BattleRepoWorld, text: String) {
    let battle = world.found_battle.as_ref().expect("no battle found");
    assert_eq!(battle.question().text(), &text);
}

#[when(regex = r#"^looking for a battle by session id "([^"]*)"$"#)]
fn when_look_for_battle(world: &mut BattleRepoWorld, session_id: String) {
    world.ensure_repo();
    let repo = world.repo.as_ref().unwrap();
    world.found_battle = repo.find(&session_id);
}

#[then("no battle is found")]
fn then_no_battle(world: &mut BattleRepoWorld) {
    assert!(world.found_battle.is_none());
}

#[then(regex = r#"^the battle outcome is "([^"]*)"$"#)]
fn then_battle_outcome(world: &mut BattleRepoWorld, outcome: String) {
    let expected = match outcome.as_str() {
        "Victory" => BattleOutcome::Victory,
        "Defeat" => BattleOutcome::Defeat,
        _ => panic!("unknown outcome: {}", outcome),
    };
    assert_eq!(world.outcome, Some(expected));
}

fn main() {
    futures::executor::block_on(BattleRepoWorld::run(
        "features/battle_repository.feature",
    ));
}
