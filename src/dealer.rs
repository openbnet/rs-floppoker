use std::collections::HashMap;
use std::fmt;

use rs_handstrength::normalize_equity;
use rs_handstrength::{Card, equity};
use crate::player::*;
use crate::deck::*;


#[derive(Debug, Clone)]
pub struct Dealer {
    pub p: Vec<Player>,
    pub deck: Deck,
    pub stage: Stages,
    pub seed: u64,
    pub button: u8,
    pub curr: u8,
    pub pot: u16,
    pub ah: ActionHistory,
    pub s_bets: Vec<SBet>,
    pub done_s_bets: Vec<SBet>,
    pub flop: [Card; 3]
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum Stages {
    PreFlop,
    Flop,
    Showdown
}
#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum ActionType {
    Fold,
    Check,
    Call,
    CallAI,
    Bet,
    BetAI,
    Raise,
    RaiseAI
}

impl fmt::Display for ActionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ActionType::Fold => write!(f, "F"),
            ActionType::Check => write!(f, "CK"),
            ActionType::Call => write!(f, "C"),
            ActionType::CallAI => write!(f, "CA"),
            ActionType::Bet => write!(f, "B"),
            ActionType::BetAI => write!(f, "BA"),
            ActionType::Raise => write!(f, "R"),
            ActionType::RaiseAI => write!(f, "RA"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Action {
    pub seat: u8,
    pub t: ActionType,
    pub value: u16
}

#[derive(Debug, Clone)]
pub struct SBet {
    pub a: usize,
    pub paid: Vec<u8>,
    pub unpaid: Vec<u8>,
    pub pp: Vec<PartialPaid>
}

#[derive(Debug, Clone)]
pub struct PartialPaid {
    pub seat: u8,
    pub amt: u16
}
#[derive(Debug, Clone, PartialEq)]
pub struct StartingBal {
    pub seat: u8,
    pub bal: u16
}
#[derive(Debug, Clone)]
pub struct ActionHistory {
    pub start_bal: Vec<StartingBal>,
    pub actions: Vec<Action>,
    pub pf: Vec<usize>,
    pub f: Vec<usize>
}

#[derive(Debug, Clone)]
pub struct SidePot {
    pub value: u16,          // Total amount in the side pot
    pub contributors: Vec<u8> // Seats of the players who contributed to the pot
}
// button doesnt move, we can just random the chips across diff hands
impl Dealer {

    // function to find the smallest player seat
    // should take in p Vec<Player>
    // should return the smallest seat number, a u8

    pub fn find_smallest_seat(p: &Vec<Player>) -> u8 {
        let mut smallest_seat = 255;
        for player in p {
            if player.seat < smallest_seat {
                smallest_seat = player.seat;
            }
        }
        smallest_seat
    }
    // Constructor: Initializes a new Dealer with an empty list of p
    pub fn new(seed: u64, p: Vec<Player>) -> Self {
        let button = Dealer::find_smallest_seat(&p);
        Dealer {
            p,
            deck: Deck::new(seed),
            stage: Stages::Showdown,
            seed: seed,
            button,
            pot: 0,
            curr: 0,
            ah: ActionHistory {
                start_bal: vec![],
                actions: vec![],
                pf: vec![],
                f: vec![]
            },
            s_bets: vec![],
            done_s_bets: vec![],
            flop: [Card::default(); 3],
        }
    }
    //  order p by seat according to the button
    // button should be last, first should be left of the button
    // there should be no return, it would modify the p Vec<Player> in place
    // sort the player seats by seat number first, then deal with the p before and after the button seperately to get the right order
    // are you able to do this without cloning the p to avoid unnecessary memory usage?
    pub fn order_p(&mut self) {
        // Sort p by seat
        self.p.sort_by_key(|p| p.seat);

        // Find the index of the player to the left of the button
        let index = self.p.iter().position(|p| p.seat == self.button)
            .expect("Button not found") + 1;

        // Rotate p so that the player to the left of the button is at the front
        let num_p = self.p.len(); 
        self.p.rotate_left(index % num_p);
    } 
  

    // new hand function
    // does not support calling a second time as it doesnt reset the hand and p states
    pub fn new_hand(&mut self) {
        self.deck = Deck::new(self.seed);
        self.order_p();
        match self.stage {
            Stages::Showdown => {
                for player in &mut self.p {
                    player.is_folded = false;
                    player.is_all_in = false;
                    player.hand = self.deck.draw4();
                    if player.chips < 1 {
                        panic!("Player does not have enough chips");
                    }
                }
            }
            _ => panic!("new_hand Hand not over"),
        };
        self.curr = self.p[0].seat;
        self.stage = Stages::PreFlop;
        self.pot = 0;
        self.flop = [Card::default(); 3];
        self.s_bets = vec![];
        self.done_s_bets = vec![];
        // update action history starting bal
        self.ah.start_bal = self.p.iter().map(|p| StartingBal {
            seat: p.seat.clone(),
            bal: p.chips.clone()
        }).collect::<Vec<StartingBal>>();

        // first player to post small blind by p_action bet 1
        // println!("new hand {:?}", self.ah.start_bal);
        if self.ah.start_bal[0].bal > 1 {
            self.p_action(Action {
                seat: self.ah.start_bal[0].seat,
                t: ActionType::Bet,
                value: 1
            });
        } else {
            self.p_action(Action {
                seat: self.ah.start_bal[0].seat,
                t: ActionType::BetAI,
                value: self.ah.start_bal[0].bal
            });
        }

        // second player to post bb
        if self.ah.start_bal[1].bal > 2 {
            self.p_action(Action {
                seat: self.ah.start_bal[1].seat,
                t: ActionType::Raise,
                value: 1
            });
        } else {
            if self.ah.start_bal[1].bal == 1 {
                self.p_action(Action {
                    seat: self.ah.start_bal[1].seat,
                    t: ActionType::CallAI,
                    value: 0
                });
            } else {
                self.p_action(Action {
                    seat: self.ah.start_bal[1].seat,
                    t: ActionType::RaiseAI,
                    value: 1
                });
            }

        }


    }

    // Removes chips from a specific player
    fn remove_chips_from_player(&mut self, seat: &u8, amt: &u16) {
        if let Some(player) = self.p.iter_mut().find(|p| &p.seat == seat) {
            player.remove_chips(amt);
        } else {
            panic!("Player not found");
        }
    }

    // Adds chips to a specific player
    fn add_chips_to_player(&mut self, seat: &u8, amt: &u16) {
        if let Some(player) = self.p.iter_mut().find(|p| &p.seat == seat) {
            player.add_chips(amt);
        } else {
            panic!("Player not found");
        }
    }
    // remove chips from player and adds it to pot
    // should take in self, seat and amt

    pub fn add_chips_to_pot(&mut self, seat: &u8, amt: &u16) {
        self.remove_chips_from_player(seat, amt);
        self.pot += amt;
    }

    // add chips to player and reduce from pot
    // should take in self, seat and amt

    pub fn pay_from_pot(&mut self, seat: &u8, amt: &u16) {
        self.add_chips_to_player(seat, amt);
        if self.pot < *amt {
            eprintln!("Pot {:?} amt {:?}", self.pot, amt);
            // println!("ah {:?}", self.ah);
            panic!("Not enough chips in pot");
        }
        self.pot -= amt;
    }
    // allows the player to make actions
    // should take in self and an action struct

    pub fn p_action(&mut self, action: Action) {
        // println!("p action {:?}", action);
        // not all rules are checked, im lazy and it runs faster without it
        // makes it hard to catch errors
        // @TODO should check for errors by parsing ActionHistory to see if rules are broken 
        if self.curr != action.seat {
            panic!("Not your turn");
        }
        if self.stage == Stages::Showdown {
            panic!("Hand is over");
        }
        let call_amt = self.get_call_amt(&action.seat);
        let p_chips = self.p.iter().find(|p| p.seat == action.seat).unwrap().chips;
        match action.t {
            ActionType::Call => {
                if call_amt > p_chips {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("Not enough chips");
                }
                if action.value != 0 {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("Call action should not have value");
                }
                // pay off outstanding bets
                if call_amt > 0 {
                    self.pay_all_outstanding_bets(&action.seat, &p_chips);
                    self.clean_s_bets();
                } else {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("Call No outstanding bets to call");
                }

                // index before push, no need to - 1
                let index = self.ah.actions.len();
                // s_bets doesnt need call actions, only bets and raises
                match self.stage {
                    Stages::PreFlop => {
                        self.ah.pf.push(index);
                    }
                    Stages::Flop => {
                        self.ah.f.push(index);
                    }
                    _ => panic!("Not implemented")
                }
                self.ah.actions.push(action);
            }
            ActionType::CallAI => {
                // there may be bets that the callAI amt is not enough to cover
                // it needs to go into the partial paid vec then
                if action.value != 0 {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("CallAI action should not have value");
                }
                if call_amt < p_chips {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("cant callAI too many chips");
                }
                // pay off outstanding bets
                // println!("call ai called {:?} {:?}", call_amt, action);
                if call_amt > 0 {
                    self.pay_all_outstanding_bets(&action.seat, &p_chips);
                    self.clean_s_bets();
                } else {
                    // println!("gona painc callAmt is 0 {:?} {:?} {:?}", self.stage, action, self.ah);
                    // println!("s_bets {:?} done_s_bets {:?}", self.s_bets, self.done_s_bets);
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("call AI No outstanding bets to call");
                }
                let player = self.p.iter_mut().find(|p| p.seat == action.seat).unwrap();
                player.is_all_in = true;
                let index = self.ah.actions.len();
                // s_bets doesnt need call actions, only bets and raises
                match self.stage {
                    Stages::PreFlop => {
                        self.ah.pf.push(index);
                    }
                    Stages::Flop => {
                        self.ah.f.push(index);
                    }
                    _ => panic!("Not implemented")
                }
                self.ah.actions.push(action);
            },
            ActionType::Fold => {
                if action.value != 0 {
                    panic!("Fold action should not have value");
                }
                // println!("fold called {:?} {:?}", call_amt, action);
                // fold
                // Re-borrow player to perform the action
                let player = self.p.iter_mut().find(|p| p.seat == action.seat).unwrap();
                player.is_folded = true;
                // index before push, no need to - 1
                let index = self.ah.actions.len();
                // s_bets doesnt need fold actions
                // need to remove folded player from all s_bets
                for s_action in &mut self.s_bets {
                    s_action.unpaid.retain(|x| x != &action.seat);
                }
                self.clean_s_bets();
                match self.stage {
                    Stages::PreFlop => {
                        self.ah.pf.push(index);
                    }
                    Stages::Flop => {
                        self.ah.f.push(index);
                    }
                    _ => panic!("Not implemented")
                }
                self.ah.actions.push(action);
                
            },
            ActionType::Check => {
                if action.value != 0 {
                    panic!("Check action should not have value");
                }
                // check
                // index before push, no need to - 1
                let index = self.ah.actions.len();
                // s_bets doesnt need check actions
                match self.stage {
                    Stages::PreFlop => {
                        self.ah.pf.push(index);
                    }
                    Stages::Flop => {
                        self.ah.f.push(index);
                    }
                    _ => panic!("Not implemented")
                }
                self.ah.actions.push(action);
            },
            ActionType::Bet => {
                if &action.value + call_amt > p_chips {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("Not enough chips");
                }
                // pay off outstanding bets
                if call_amt > 0 {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("Bet should not have outstanding bets");
                } 
                // pay the bet
                self.add_chips_to_pot(&action.seat, &action.value);

                let not_player: Vec<u8> = self.p.iter().filter(|p| p.seat != action.seat && !p.is_all_in && !p.is_folded).map(|p| p.seat).collect::<Vec<u8>>();
                // index before push, no need to - 1
                let index = self.ah.actions.len();
                self.s_bets.push(SBet {
                    a: index,
                    paid: vec![action.seat],
                    unpaid: not_player,
                    pp: vec![]
                });
                match self.stage {
                    Stages::PreFlop => {
                        self.ah.pf.push(index);
                    }
                    Stages::Flop => {
                        self.ah.f.push(index);
                    }
                    _ => panic!("Not implemented")
                }
                self.ah.actions.push(action);

            },
            ActionType::BetAI => {

                if &action.value != &p_chips {
                    eprintln!("action value {:?} p_chips {:?}", action.value, p_chips);
                    panic!("BetAI should bet all in");
                }
                // pay off outstanding bets
                if call_amt > 0 {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("Bet should not have outstanding bets");
                } 
                // pay the bet
                self.add_chips_to_pot(&action.seat, &action.value);
                // set player all in

                let not_player: Vec<u8> = self.p.iter().filter(|p| p.seat != action.seat && !p.is_all_in && !p.is_folded).map(|p| p.seat).collect::<Vec<u8>>();
                let player = self.p.iter_mut().find(|p| p.seat == action.seat).unwrap();
                player.is_all_in = true;

                // index before push, no need to - 1
                let index = self.ah.actions.len();
                self.s_bets.push(SBet {
                    a: index,
                    paid: vec![action.seat],
                    unpaid: not_player,
                    pp: vec![]
                });
                match self.stage {
                    Stages::PreFlop => {
                        self.ah.pf.push(index);
                    }
                    Stages::Flop => {
                        self.ah.f.push(index);
                    }
                    _ => panic!("flop poker wont have actions past flop")
                }
                self.ah.actions.push(action);

            }
            ActionType::Raise => {
                if &action.value + call_amt > p_chips {
                    // println!("raise called {:?} {:?}", call_amt, action);
                    // println!("pchips {:?} pot {:?}", p_chips, self.pot);
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("raise Not enough chips");
                }
                if &action.value > &(call_amt + self.pot) {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    panic!("raise too much");
                }
                // println!("raise called {:?} {:?}", call_amt, action);
                // pay off outstanding bets
                if call_amt > 0 {
                    // println!("paying off outstanding bets {:?} {:?}", call_amt, p_chips);
                    self.pay_all_outstanding_bets(&action.seat, &p_chips);
                    self.clean_s_bets();
                    // println!("after {:?} {:?}", call_amt, self.p.iter().find(|p| p.seat == action.seat).unwrap().chips)
                }
                // println!("paying off raise now {:?} {:?}", action, self.s_bets); 
                // pay the bet
                self.add_chips_to_pot(&action.seat, &action.value);

                let not_player: Vec<u8> = self.p.iter().filter(|p| p.seat != action.seat && !p.is_all_in && !p.is_folded).map(|p| p.seat).collect::<Vec<u8>>();
                // index before push, no need to - 1
                let index = self.ah.actions.len();
                self.s_bets.push(SBet {
                    a: index,
                    paid: vec![action.seat],
                    unpaid: not_player,
                    pp: vec![]
                });
                match self.stage {
                    Stages::PreFlop => {
                        self.ah.pf.push(index);
                    }
                    Stages::Flop => {
                        self.ah.f.push(index);
                    }
                    _ => panic!("Not implemented")
                }
                self.ah.actions.push(action);

            },
            ActionType::RaiseAI => {
                if &action.value + call_amt != p_chips {
                    eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?} call_amt {:?}", action, call_amt);
                    panic!("raiseai incorrect chips");
                }
                if &action.value > &(call_amt + self.pot) {
                                        eprintln!("debug {:#?}", self);
                    eprintln!("action {:#?}", action);
                    
                    panic!("raiseAI too much");
                }
                // println!("raise called {:?} {:?}", call_amt, action);
                // println!("pchips {:?} pot {:?}", p_chips, self.pot);
                // pay off outstanding bets
                if call_amt > 0 {
                    // println!("paying off outstanding bets {:?}", call_amt);
                    self.pay_all_outstanding_bets(&action.seat, &p_chips);
                    self.clean_s_bets();
                }
                // println!("paying off raise now {:?} {:?}", action, self.s_bets); 
                // pay the bet
                self.add_chips_to_pot(&action.seat, &action.value);



                let not_player: Vec<u8> = self.p.iter().filter(|p| p.seat != action.seat && !p.is_all_in && !p.is_folded).map(|p| p.seat).collect::<Vec<u8>>();
                let player = self.p.iter_mut().find(|p| p.seat == action.seat).unwrap();
                player.is_all_in = true;
                // index before push, no need to - 1
                let index = self.ah.actions.len();
                self.s_bets.push(SBet {
                    a: index,
                    paid: vec![action.seat],
                    unpaid: not_player,
                    pp: vec![]
                });
                match self.stage {
                    Stages::PreFlop => {
                        self.ah.pf.push(index);
                    }
                    Stages::Flop => {
                        self.ah.f.push(index);
                    }
                    _ => panic!("Not implemented")
                }
                self.ah.actions.push(action);

            }
        }
        
        self.update_stage();
    }
    // function to decide if the stage has ended and to update stage and curr accordingly
    // should take in self, self.ah and self.s_bets has been updated but not self.curr
    // should not return anything and just update the stage if needed and the curr
    // should follow omaha poker rules to decide if the stage has ended
    // if there is nothing in self.s_bets, it means that either nobody has acted, or the players have all checked previously
    // this can never happen on preflop, since there is the sb and bb bets.
    // if there is something in self.s_bets, it means that there are outstanding bets
    // if there are outstanding bets, the stage has not ended
    // we need to handle the check around situations on the turn
    // we also need to check for active players who are not folded and not all in, and if there is only 1 active player, we proceed to showdown
    // first check if self.s_bets is empty
    // if it is empty, check if there are any active players who are not folded and not all in

    pub fn update_stage(&mut self) {
        let s_bets_len = self.s_bets.len();
        let ap_count = self.p.iter().filter(|p| !p.is_folded && !p.is_all_in).count();
        // println!("update stage {:?} {:?}", self.stage, self.s_bets);
        // println!("ap count {:?}", ap_count);
        if s_bets_len == 0 {
            if self.stage == Stages::PreFlop {
                // deal 3 cards to the flop
                // we need to handle for the big blind having the option to check or bet
                // println!("pre flop s_bets {:?} all_s_bets {:?}", self.s_bets, self.done_s_bets);
                let num_a = self.ah.actions.len();
                let last_action = &self.ah.actions[num_a - 1];
                let is_last_action_bb_check = 
                    last_action.t == ActionType::Check &&
                    last_action.seat == self.ah.start_bal[1].seat;

                if !is_last_action_bb_check && self.done_s_bets.len() == 2 && ap_count > 1 {
                    // the last bet was the bb
                    self.next_player(); 
                    return;
                } else {
                    // println!("gona deal flop");
                    if self.s_bets.len() != 0 {
                        panic!("s_bets should be empty");
                    }
                    self.deal_flop();
    
                    // println!("dealt flop {:?}", ap_count);
                    if ap_count <= 1 {
                        self.stage = Stages::Showdown;
                        return;
                    } else {
                        self.stage = Stages::Flop;
                        self.curr = self.button;
                        self.next_player();
                        return;
                    }

                }

            } else {
                // println!("flop s_bets {:?} all_s_bets {:?}", self.s_bets, self.done_s_bets);
                if ap_count <= 1 {
                    self.stage = Stages::Showdown;
                    return;
                } else {
                    let flop_actions: &[Action] = &self.ah.actions[self.ah.f[0]..];
                    // println!("flop actions {:?}", self.ah);
                    let is_latest_action_check = flop_actions.iter().last().unwrap().t == ActionType::Check;
                    // println!("is latest action check {:?}", is_latest_action_check);
                    if is_latest_action_check {
                        // check if all players have checked
                        let num_checks = flop_actions.iter().filter(|a| a.t == ActionType::Check).count();
                        // println!("num checks {:?} ap {:?}", num_checks, ap_count);
                        if num_checks > ap_count {
                            eprintln!("flop actions {:?}", flop_actions);
                            eprintln!("ah {:#?}", self.ah);
                            panic!("too many checks");
                        }
                        if num_checks == ap_count {
                            self.stage = Stages::Showdown;
                        } else {
                            self.next_player();
                        }
                    } else {
                        self.stage = Stages::Showdown;
                        return;
                    }
                }
  
            }
        } else {
            // println!("s_bets len not 0 {:?} {:?}", self.s_bets, self.done_s_bets);
            self.next_player();
        }
    }


    // function to update self.curr to the next player
    // it should skip over the players who are folded or all in
    // should take in self

    pub fn next_player(&mut self) {
        let curr_seat = self.curr;
        let num_p = self.p.len() as u8;
        let mut next = curr_seat % num_p + 1;
        while self.p.iter().any(|p| p.seat == next && (p.is_folded || p.is_all_in)) {
            next = next % num_p + 1;
        }

        if next == curr_seat && !(
            self.done_s_bets.len() == 2 && self.stage == Stages::PreFlop 
        ){
            panic!("All players have folded or are all in. stage {:?} s_bets {:?} done_bets {:?}", self.stage, self.s_bets, self.done_s_bets);
        }

        self.curr = next;
    }


    // get_call_amt func to get the amount to call based on s_bets and supplied seat
    // should take in self and a seat number
    // should return the amount to call, a u16
    // should check if the user's seat is in the s_bets unpaid vec
    // should calculate the amt based on the value of all outstanding bets that the user is in the unpaid vec of s_bets
    // should return 0 if the user is not in all the unpaid vec of s_bets

    pub fn get_call_amt(&self, seat: &u8) -> u16 {
       let mut total: u16 = 0;
         for action in &self.s_bets {
              if action.unpaid.contains(seat) {
                total += self.ah.actions[action.a as usize].value;
              }
         }
         total
    }


    // pay all outstanding bets func to pay all outstanding bets for a given seat
    // should take in self and a seat number
    // action should be removed from s_bets if unpaid vec is empty after the operation

    pub fn pay_all_outstanding_bets(&mut self, seat: &u8, pchips: &u16) {
        // need to pay all outstanding bets
        // need to remove seat from unpaid vec
        // need to remove action from s_bets if unpaid vec is empty after the operation
        // if user doesnt have enough chips to pay, it should be recorded in the partial paid vec
        // user should be removed from unpaid, and NOT put into paid

        let mut total: u16 = 0;
        let mut i_chips = pchips.clone();
        // println!("pay outstanding s_bets {:?} {:?}", seat, self.s_bets);
        for action in &mut self.s_bets {
            if action.unpaid.contains(seat) {

                let a_value = self.ah.actions[action.a as usize].value;
                // println!("paying seat {:?} action {:?} a_value {:?} i_chips {:?}", seat, action, a_value, i_chips);
                if i_chips < a_value {
                    if i_chips != 0 {
                        action.pp.push(PartialPaid {
                            seat: *seat,
                            amt: i_chips
                        });
                        i_chips = 0;
                        total += i_chips;
                    }
                    action.unpaid.retain(|x| x != seat);
                } else {
                    total += a_value;
                    i_chips -= a_value;
                    action.unpaid.retain(|x| x != seat);
                    action.paid.push(*seat);
                }

            }
        }
        // println!("add_chips_to_pot{:?} {:?}", seat, total);
        self.add_chips_to_pot(seat, &total);
    }

    // function to check thru all s_bets to see if there are any with unpaid len == 0
    // should clone s_bets to all_s_bets and remove it from s_bets
    // it should be able to handle multiple actions that need to be cleaned at the same time

    pub fn clean_s_bets(&mut self) {
        let mut must_clean_index: Vec<usize> = vec![];
        for (i, action) in self.s_bets.iter().enumerate() {
            if action.unpaid.len() == 0 {
                must_clean_index.push(i);
            }
        }
        // println!("clean s_bets {:?} {:?}", must_clean_index, self.s_bets);
        let mut num_deleted = 0;
        for i in must_clean_index {
            // push s_bet to self.all_s_bets then remove it
            // println!("clean bets i {:?} num_deleted {:?}", i, num_deleted);
            let y = i - num_deleted;
            let mut clone = self.s_bets[y].clone();
            clone.paid.sort();
            self.done_s_bets.push(clone);
            self.s_bets.remove(y);
            num_deleted += 1;

        }
        
    }
    // handle showdown
    // should take in self
    // should use rs_handstrength::equity to calculate the equity of each player
    // pay out the pot by equity to each player
    // should not return anything

    pub fn handle_showdown(&mut self) {
        if self.stage != Stages::Showdown {
            panic!("showdown Hand not over");
        }
        // we need to clone ah and replace it, below mutates ah.actions which sux for re-running hands
        let tmp_actions = self.ah.actions.clone();
        // println!("handle showdown done s bets {:?}", self.done_s_bets);
        self.refund_excess();
        // println!("after refund done s bets {:?}", self.done_s_bets);
        // println!("ah {:?}", self.ah);
        // println!("after refund actions {:?}", self.ah.actions);
        let showdown_players_seats: Vec<u8> = self.p.iter().filter(|p| !p.is_folded).map(|p| p.seat).collect::<Vec<u8>>();
        let pot = self.pot.clone();
    
        if showdown_players_seats.len() == 0 {
            // println!("ah {:?}", self.ah);
            panic!("showdown No players left");
        } else if showdown_players_seats.len() == 1 {
            // only 1 player left, pay out the pot
            let winner = showdown_players_seats[0];
            // println!("only one player {:?}", winner);
            self.pay_from_pot(&winner, &pot);
        } else {
            if self.flop[0].value == 0 {
                panic!("Flop not dealt");
            }
            let showdown_player_hands: Vec<[Card; 4]> = self.p.iter().filter(|p| !p.is_folded).map(|p| p.hand).collect::<Vec<[Card; 4]>>();
            // println!("showdown player hands {:?}", showdown_player_hands);
            // println!("showdown pseats {:?}", showdown_players_seats);
            let equities = normalize_equity(&equity(&showdown_player_hands, &self.flop));
            // println!("equities {:?}", equities);
            
            let sidepots = self.group_side_pots();
            // println!("sidepots {:?}", sidepots);
            for sidepot in sidepots {
                for (i, equity) in equities.iter().enumerate() {
                    let sidepot_total = sidepot.value * sidepot.contributors.len() as u16;
                    let chips = (*equity as f32 / 100 as f32 * sidepot_total as f32).floor() as u16;
                    // println!("pot {:?} sidepot_total {:?} equity {:?} chips {:?}", self.pot, sidepot_total, equity, chips);
                    let min = std::cmp::min(chips, self.pot);
                    self.pay_from_pot(&showdown_players_seats[i], &min);
                }
            }

            if self.pot > 0 {
                if self.pot > self.p.len() as u16 * 4 && self.pot as f32 / pot as f32 > 0.2 {
                    // println!("ah {:?}", self.ah);
                    panic!("isnt remainder pot {:?}", self.pot);
                }
                // println!("has remaining pot {:?}", self.pot);
                self.pay_from_pot(&showdown_players_seats[0], &self.pot.clone());
            }
        }

        self.ah.actions = tmp_actions;
    }

    // helper function to deal 3 cards to the self.flop
    // should take in self
    // should return nothing

    pub fn deal_flop(&mut self) {
        if self.stage != Stages::PreFlop {
            panic!("Not preflop");
        }
        self.flop = self.deck.draw3();
    }

    pub fn group_side_pots(&self) -> Vec<SidePot> {
        // println!("group side pots called {:?}", &self.done_s_bets);
        let mut side_pots: HashMap<Vec<u8>,SidePot> = HashMap::new(); 
        // first break up all the partial paid into their own bets
        // to break it up you
            // first find the smallest partial paid value
            // deduct smallestValue from that action's value and ALL its partial paid values
            // create a new action with the smallestValue, with its old paid vec + ALL partial paid seats
            // run the loop until there is no more partial paid
            // this bascially breaks the bet/raise into multiple smaller bets/raises that have NO partial paids.
            // we dont care about the unpaid field at this point
        // then we need merge all the bets that have the same sorted paid vec

       
        for bet in &self.done_s_bets {
            if bet.pp.len() != 0 {
                // Partially paid bets
                let mut ppbet = bet.clone();
                let mut ppbet_value = self.ah.actions[ppbet.a].value;
                // println!("inital ppbet {:?} ppbet_value {:?}", ppbet, ppbet_value);
                while ppbet.pp.len() != 0 {
                   
                    let smallest_value = bet.pp.iter().min_by(|a, b| a.amt.cmp(&b.amt)).unwrap().amt;
                    // println!("found pp {:?} smallest value {:?}", bet, smallest_value);
                    let mut new_paid: Vec<u8> = bet.paid.clone();
                    new_paid.extend(bet.pp.iter().map(|pp| pp.seat));
                    new_paid.sort();
                    side_pots.entry(new_paid.clone())
                        .and_modify(|e| e.value += smallest_value)
                        .or_insert(SidePot { value: smallest_value, contributors: new_paid }); 

                    ppbet.pp = ppbet.pp.iter().map(|pp| PartialPaid {
                        seat: pp.seat,
                        amt: pp.amt - smallest_value
                    }).filter(|pp| pp.amt != 0).collect::<Vec<PartialPaid>>();
                    ppbet_value -= smallest_value;
                }
                // println!("ppbet {:?} ppbet_value {:?}", ppbet, ppbet_value);
                // add original bet to side pots
                let mut new_paid: Vec<u8> = bet.paid.clone();
                new_paid.sort();
                side_pots.entry(new_paid.clone())
                    .and_modify(|e| e.value += ppbet_value)
                    .or_insert(SidePot { value: ppbet_value, contributors: new_paid });

            } else {
                // Fully paid bets
                let mut new_paid: Vec<u8> = bet.paid.clone();
                new_paid.sort();
                side_pots.entry(new_paid.clone())
                    .and_modify(|e| e.value += self.ah.actions[bet.a].value)
                    .or_insert(SidePot { value: self.ah.actions[bet.a].value, contributors: new_paid });
            }

 
        }
        // println!("side pots {:?}", side_pots);
        side_pots.values().cloned().collect()
    }

    pub fn refund_excess(&mut self) {
        if self.stage != Stages::Showdown {
            panic!("refund Hand not over");
        }
        // println!("refund excess done s bets {:?}", self.done_s_bets);
        // Extract necessary data from the latest bet
        let (latest_bet_a, latest_bet_paid, latest_bet_pp, latest_bet_value) = {
            let latest_bet = self.done_s_bets.last().unwrap();
            (
                latest_bet.a,
                latest_bet.paid.clone(),
                latest_bet.pp.clone(),
                self.ah.actions[latest_bet.a].value
            )
        };
        // println!("refund excess called {:?} {:?} {:?} {:?}", latest_bet_a, latest_bet_paid, latest_bet_pp, latest_bet_value);
        if latest_bet_paid.len() == 1 {
            let player = latest_bet_paid[0];
            if latest_bet_pp.is_empty() {
                // refund whole bet to player
                self.pay_from_pot(&player, &latest_bet_value);
                // remove this action from done_s_bets
                self.done_s_bets.pop();
            } else {
                // Make the highest partial paid value the new action value
                // Refund the old value - highest partial paid to action.seat
                let highest_pp = latest_bet_pp.iter()
                    .max_by(|a, b| a.amt.cmp(&b.amt))
                    .unwrap().amt;
                // Refund excess to player
                let excess = latest_bet_value - highest_pp;
                self.pay_from_pot(&player, &excess);
                // println!("refund excess pp {:?} {:?}", player, excess);
                // Mutate the action value
                self.ah.actions[latest_bet_a].value = highest_pp;
    
 
                // Add the highest value entries to the paid vec
                self.done_s_bets.last_mut().unwrap().paid
                    .extend(latest_bet_pp.iter()
                        .filter(|x| x.amt == highest_pp)
                        .map(|x| x.seat));

                // Mutate the done_s_bet last pp so that the highest value entries are removed
                // The lower value entries are kept as is
                self.done_s_bets.last_mut().unwrap().pp
                    .retain(|x| x.amt != highest_pp);
            }
        }
    }

    // helper function for lib users to get available actions for current player
    // should take in self
    // should return a Vec<ActionType> of available actions for the current player

    pub fn get_available_actions(&self) -> Vec<ActionType> {
        if self.stage == Stages::Showdown {
            panic!("Hand is over cant get_available_actions");
        }
        let mut available_actions: Vec<ActionType> = vec![];
        let call_amt = self.get_call_amt(&self.curr);
        let p_chips = self.p.iter().find(|p| p.seat == self.curr).unwrap().chips;
        if self.s_bets.len() == 0 {
            // always check or bet, if can bet all in, always bet all in
            available_actions.push(ActionType::Check);
            if p_chips <= self.pot {
                available_actions.push(ActionType::BetAI);
            } else {
                available_actions.push(ActionType::Bet);
            }
        } else {
            // has bet, so you can only call fold or raise
            available_actions.push(ActionType::Fold);
            if call_amt >= p_chips {
                available_actions.push(ActionType::CallAI);
            } else {
                available_actions.push(ActionType::Call);
                // let min raise = call_amt 
                let active_players = self.p.iter().filter(|p| !p.is_folded && !p.is_all_in).count();
                if active_players > 1 {
                    // not counting the bb raise, there can be a max of 3 raises preflop
                    // and a max of 4 raises postflop
                    // println!("s_bets {:?} done_s_bets {:?}", self.s_bets, self.done_s_bets);
                    let is_flop_stage = self.stage == Stages::Flop;
                    let max_raise = 4; 
                    let mut num_raises = 0;

                    // Combine the iterations
                    for sb in self.s_bets.iter().chain(self.done_s_bets.iter()
                        .filter(|&sb| {
                            // Skip specific checks for Flop stage in done_s_bets
                            if is_flop_stage && self.ah.pf.contains(&sb.a) {
                                return false;
                            }
                            true
                        })
                
                    ) {

                        let at = &self.ah.actions[sb.a as usize].t;
                        if *at == ActionType::Raise || *at == ActionType::RaiseAI {
                            num_raises += 1;
                        }
                    }

                    if num_raises > max_raise {
                        if p_chips < call_amt + self.pot {
                            available_actions.push(ActionType::RaiseAI);
                        }
                       // we ignore for num_raises > max_raise and too many chips to raiseAI
                    } else {
                        if p_chips < call_amt + self.pot {
                            available_actions.push(ActionType::RaiseAI);
                        } else {
                            // cant raise if you can raiseAI
                            available_actions.push(ActionType::Raise);
                        }
                        
                    }
                    

                }

            }
            
        }

        // println!("available actions {:?}", available_actions);
        available_actions 
    } 
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_hand() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 15),
            Player::new(2, 12),
            Player::new(3, 10),
        ]);
        let init_player_chips = &dealer.p.iter().map(|p| p.chips).collect::<Vec<u16>>();
        assert_eq!(init_player_chips, &vec![15, 12, 10]);
        dealer.new_hand();

        for player in &dealer.p {
            assert_eq!(player.is_folded, false);
            assert_eq!(player.is_all_in, false);
            assert_eq!(player.hand.len(), 4);
        }
        let player_order = &dealer.p.iter().map(|p| p.seat).collect::<Vec<u8>>();
        assert_eq!(player_order, &vec![2, 3, 1]);

        let player_chips = &dealer.p.iter().map(|p| p.chips).collect::<Vec<u16>>();
        assert_eq!(player_chips, &vec![11, 8, 15]);
        assert_eq!(dealer.stage, Stages::PreFlop);
        assert_eq!(dealer.curr, 1);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::Call, ActionType::Raise]);
        // player 1 to call
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.curr, 2);
        assert_eq!(dealer.pot, 5);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::Call, ActionType::Raise]);
        // player 2 to call
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Call,
            value: 0
        });

        assert_eq!(dealer.stage, Stages::PreFlop);
        assert_eq!(dealer.curr, 3);
        assert_eq!(dealer.pot, 6);
        // p3 bb to open a new round of betting preflop
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Check, ActionType::Bet]);
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Bet,
            value: 6
        });
        assert_eq!(dealer.stage, Stages::PreFlop);
        assert_eq!(dealer.curr, 1);
        assert_eq!(dealer.pot, 12);
        // p1 to call
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold , ActionType::Call, ActionType::RaiseAI]);
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::PreFlop);
        assert_eq!(dealer.curr, 2);
        assert_eq!(dealer.pot, 18);
        // p2 to call
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold , ActionType::Call, ActionType::RaiseAI]);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::Flop);
        assert_eq!(dealer.curr, 2);
        assert_eq!(dealer.pot, 24);
        // p2 to check
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Check , ActionType::BetAI]);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Check,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::Flop);
        assert_eq!(dealer.curr, 3);
        assert_eq!(dealer.pot, 24);
        // p3 to bet all in
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Check , ActionType::BetAI]);
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::BetAI,
            value: 2
        });
        assert_eq!(dealer.p[1].is_all_in, true);
        assert_eq!(dealer.stage, Stages::Flop);

        // // p1 to raiseAI
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::Call, ActionType::RaiseAI]);
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::RaiseAI,
            value: 5
        });
        assert_eq!(dealer.stage, Stages::Flop);
        assert_eq!(dealer.curr, 2);
        assert_eq!(dealer.pot, 33);
        // println!(" DOING FINAL CALL !!!!!!!!");
        // // // p2 to callAI
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold , ActionType::CallAI]);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::CallAI,
            value: 0
        }); 
        // println!("s_bets {:?}", dealer.s_bets);
        // println!("done_s_bets {:?}", dealer.done_s_bets);
        // println!("ah {:?}", dealer.ah.actions);
        let done_s_bet_indexes = &dealer.done_s_bets.iter().map(|s| s.a).collect::<Vec<usize>>();
        assert_eq!(done_s_bet_indexes, &vec![0, 1, 4, 8, 9]); 
        // println!("ah {:?}", dealer.ah);
        assert_eq!(dealer.stage, Stages::Showdown);
        // // execute showdown
        dealer.handle_showdown();
        assert_eq!(dealer.stage, Stages::Showdown);
        assert_eq!(dealer.pot, 0);


    }
    // tests that tests 3 players with 10 chips, seat one is the dealer and would raise 5 chips, seat 2 would raise AI and seat 3 will call allIN
    // it should hit the showdown stage

    #[test]
    fn test_ai() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 15),
            Player::new(2, 12),
            Player::new(3, 10),
        ]);

        dealer.new_hand();

        for player in &dealer.p {
            assert_eq!(player.is_folded, false);
            assert_eq!(player.is_all_in, false);
            assert_eq!(player.hand.len(), 4);
        }
        let player_order = &dealer.p.iter().map(|p| p.seat).collect::<Vec<u8>>();
        assert_eq!(player_order, &vec![2, 3, 1]);

        let player_chips = &dealer.p.iter().map(|p| p.chips).collect::<Vec<u16>>();
        assert_eq!(player_chips, &vec![11, 8, 15]);
        assert_eq!(dealer.pot, 3);
        assert_eq!(dealer.stage, Stages::PreFlop);
        assert_eq!(dealer.curr, 1);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::Call, ActionType::Raise]);
        // p1 to raise 5
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Raise,
            value: 5
        });
        assert_eq!(dealer.curr, 2);
        assert_eq!(dealer.pot, 10);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::Call, ActionType::RaiseAI]);

        // p2 to raiseAI
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::RaiseAI,
            value: 5
        });
        assert_eq!(dealer.curr, 3);
        assert_eq!(dealer.pot, 21);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::CallAI]);
        // 3 to callAI
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::CallAI,
            value: 0
        });
        assert_eq!(dealer.curr, 1);
        assert_eq!(dealer.pot, 29);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::Call]);
        // 1 to call
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.pot, 34);
        //showdown
        assert_eq!(dealer.stage, Stages::Showdown);
        dealer.handle_showdown();
        assert_eq!(dealer.stage, Stages::Showdown);
        


    }

    // tests that 3 players each have 2 chips to begin with and 1 is the dealer.

    #[test]
    fn test_2chips() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 2),
            Player::new(2, 2),
            Player::new(3, 2),
        ]);

        dealer.new_hand();

        let player_order = &dealer.p.iter().map(|p| p.seat).collect::<Vec<u8>>();
        assert_eq!(player_order, &vec![2, 3, 1]);

        let player_chips = &dealer.p.iter().map(|p| p.chips).collect::<Vec<u16>>();
        assert_eq!(player_chips, &vec![1, 0, 2]);
        assert_eq!(dealer.pot, 3);
        assert_eq!(dealer.stage, Stages::PreFlop);
        assert_eq!(dealer.curr, 1);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::CallAI]);

        // 1 to callAI
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::CallAI,
            value: 0
        });
        assert_eq!(dealer.curr, 2);
        assert_eq!(dealer.pot, 5);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Fold, ActionType::CallAI]);
        // // 2 to fold
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Fold,
            value: 0
        });   
        assert_eq!(dealer.stage, Stages::Showdown);
        dealer.handle_showdown();
        assert_eq!(dealer.stage, Stages::Showdown);
        assert_eq!(dealer.pot, 0);

     }
    #[test]
    fn test_3chips() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 3),
            Player::new(2, 3),
            Player::new(3, 2),
        ]);

        dealer.new_hand();

        let player_order = &dealer.p.iter().map(|p| p.seat).collect::<Vec<u8>>();
        assert_eq!(player_order, &vec![2, 3, 1]);

        let player_chips = &dealer.p.iter().map(|p| p.chips).collect::<Vec<u16>>();
        assert_eq!(player_chips, &vec![2, 0, 3]);
        assert_eq!(dealer.pot, 3);
        assert_eq!(dealer.stage, Stages::PreFlop);
        assert_eq!(dealer.curr, 1);
        assert_eq!(dealer.get_available_actions(), 
        vec![ActionType::Fold, ActionType::Call, ActionType::RaiseAI]);
    // 1 to raise ai
    dealer.p_action(Action {
        seat: 1,
        t: ActionType::RaiseAI,
        value: 1
    });
    assert_eq!(dealer.curr, 2);
    assert_eq!(dealer.pot, 6);
    assert_eq!(dealer.get_available_actions(), 
        vec![ActionType::Fold, ActionType::CallAI]);


    }

    #[test]
    fn test_sb_folds() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 5),
            Player::new(2, 5),
            Player::new(3, 5),
        ]);

        dealer.new_hand();
        // 1 calls
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Call,
            value: 0
        });
        // sb 2 folds
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Fold,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::PreFlop);
        assert_eq!(dealer.curr, 3);
        assert_eq!(dealer.get_available_actions(), vec![ActionType::Check, ActionType::BetAI]);
        // bb checks
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Check,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::Flop);
        assert_eq!(dealer.curr, 3);
 
    }
    #[test]
    fn test_1folds() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 100),
            Player::new(2, 100),
            Player::new(3, 100),
        ]);

        dealer.new_hand();
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Fold,
            value: 0
        });
        assert_eq!(dealer.curr, 2);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::PreFlop);
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Check,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::Flop);
        assert_eq!(dealer.curr, 2);
        


    }

    // test this action history
    //  ActionHistory { start_bal: [StartingBal { seat: 2, bal: 100 }, StartingBal { seat: 3, bal: 100 }, StartingBal { seat: 4, bal: 100 }, StartingBal { seat: 1, bal: 100 }], 
    // actions: [Action { seat: 2, t: Bet, value: 1 }, Action { seat: 3, t: Raise, value: 1 }, 
    // Action { seat: 4, t: Call, value: 0 }, Action { seat: 1, t: Fold, value: 0 }, Action { seat: 2, t: Call, value: 0 }, Action { seat: 3, t: Check, value: 0 }, 
    // flop Action { seat: 2, t: Check, value: 0 }, Action { seat: 3, t: Check, value: 0 }, 
    // Action { seat: 4, t: Bet, value: 3 }, Action { seat: 2, t: Call, value: 0 }, 
    // Action { seat: 3, t: Raise, value: 15 }, Action { seat: 4, t: Raise, value: 14 }, Action { seat: 2, t: Raise, value: 58 }, 
    // Action { seat: 3, t: Call, value: 0 }, Action { seat: 4, t: RaiseAI, value: 8 }, Action { seat: 2, t: CallAI, value: 0 }, Action { seat: 3, t: Fold, value: 0 }], pf: [0, 1, 2, 3, 4, 5], f: [6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16] }
    
    #[test]
    fn test_4players() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 100),
            Player::new(2, 100),
            Player::new(3, 100),
            Player::new(4, 100),
        ]);

        dealer.new_hand();

        // 4 calls
        dealer.p_action(Action {
            seat: 4,
            t: ActionType::Call,
            value: 0
        });
        // 1 folds
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Fold,
            value: 0
        });
        // 2 calls
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Call,
            value: 0
        });
        // 3 checks
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Check,
            value: 0
        });
        // flop
        assert_eq!(dealer.stage, Stages::Flop);
        assert_eq!(dealer.curr, 2);
        // 2 checks
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Check,
            value: 0
        });
        // 3 checks
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Check,
            value: 0
        });
        // 4 bets 3
        dealer.p_action(Action {
            seat: 4,
            t: ActionType::Bet,
            value: 3
        });
        // 2 calls
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Call,
            value: 0
        });
        // 3 raise 15
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Raise,
            value: 15
        });
        // 4 raise 14
        dealer.p_action(Action {
            seat: 4,
            t: ActionType::Raise,
            value: 14
        });
        // 2 raise 58
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Raise,
            value: 58
        });
        // 3 calls
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Call,
            value: 0
        });
        // 4 raiseAI 8
        dealer.p_action(Action {
            seat: 4,
            t: ActionType::RaiseAI,
            value: 8
        });
        // 2 callAI
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::CallAI,
            value: 0
        });
        // 3 folds
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Fold,
            value: 0
        });
        // showdown
        assert_eq!(dealer.stage, Stages::Showdown);
        dealer.handle_showdown();
        assert_eq!(dealer.stage, Stages::Showdown);
        assert_eq!(dealer.pot, 0);



    }
    #[test]
    fn test_diff_chips() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 5),
            Player::new(2, 10),
            Player::new(3, 15),
        ]);

        dealer.new_hand();
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.curr, 2);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::PreFlop);
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Check,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::Flop);
        assert_eq!(dealer.curr, 2);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Bet,
            value: 6
        }); 
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::RaiseAI,
            value: 7
        }); 
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::CallAI,
            value: 0
        });
        // println!("before doing fold {:#?}", dealer);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Fold,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::Showdown);
        dealer.handle_showdown();
        // println!("phands {:?}", dealer.p.iter().map(|p| p.hand).collect::<Vec<[Card; 4]>>());

    }

    #[test]
    fn test_diff_chips2() {
        let mut dealer = Dealer::new(123, vec![
            Player::new(1, 5),
            Player::new(2, 10),
            Player::new(3, 15),
        ]);

        dealer.new_hand();
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.curr, 2);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::PreFlop);
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Check,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::Flop);
        assert_eq!(dealer.curr, 2);
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Check,
            value: 0
        }); 
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Check,
            value: 0
        }); 
        dealer.p_action(Action {
            seat: 1,
            t: ActionType::BetAI,
            value: 3
        });
        dealer.p_action(Action {
            seat: 2,
            t: ActionType::Call,
            value: 0
        });
        dealer.p_action(Action {
            seat: 3,
            t: ActionType::Call,
            value: 0
        });
        assert_eq!(dealer.stage, Stages::Showdown);
        dealer.handle_showdown();
        // println!("phands {:?}", dealer.p.iter().map(|p| p.hand).collect::<Vec<[Card; 4]>>());

    }

}
