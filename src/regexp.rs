#![allow(unused, dead_code)]

use std::rc::Rc;

#[derive(Debug)]
pub enum ReErrorKind {
	NonAsciiChar,
	BadSplit,
	OutOfRange,
	InvalidChar,
	InvalidInt,
	InvalidRange
}

#[derive(Debug)]
pub struct ReErrorInfo {
	at: usize,
	msg: String
}

pub type ReError = (ReErrorKind, ReErrorInfo);

pub enum Re {
	Char(char),
	Or(Rc<Re>, Rc<Re>),
	And(Rc<Re>, Rc<Re>),
	Kleen(Rc<Re>),
	OneOrMore(Rc<Re>),
	Repeat(Rc<Re>, usize, usize),
	AnyChar
}

impl Re {
	pub fn parse_regexp(string: &str) -> Result<Self, ReError> {
		let bytes = string.as_bytes();
		match first_non_ascii(&bytes) {
			Some(index) => return Err((
					ReErrorKind::NonAsciiChar,
					ReErrorInfo{at: index, msg: String::from("")}
				)),
			None => ()
		}

		match parse_or(&bytes, 0) {
			Ok((reg, _)) => Ok(reg),
			Err(err) => Err(err)
		}
	}

	pub fn match_one(&self, string: &str) -> String {
		String::from("Nothing")
	}

	pub fn is_equal(&self, other: &Self) -> bool {
		match self {
			Re::Char(c1) => match other {
				Re::Char(c2) => c1 == c2,
				_ => false
			},
			Re::Kleen(c1) => match other {
				Re::Kleen(c2) => c1.is_equal(c2),
				_ => false
			},
			Re::Or(c11, c12) => match other {
				Re::Or(c21, c22) => c11.is_equal(c21) && c12.is_equal(c22),
				_ => false
			},
			Re::And(c11, c12) => match other {
				Re::And(c21, c22) => c11.is_equal(c21) && c12.is_equal(c22),
				_ => false
			},
			Re::OneOrMore(c1) => match other {
				Re::OneOrMore(c2) => c1.is_equal(c2),
				_ => false
			},
			Re::AnyChar => match other {
				Re::AnyChar => true,
				_ => false
			},
			Re::Repeat(c1, a1, b1) => match other {
				Re::Repeat(c2, a2, b2) => c1.is_equal(c2) && a1 == a2 && b1 == b2,
				_ => false
			},
		}
	}

	pub fn debug_print(&self) {
		match self {
			Re::Char(c) => print!("{}", c),
			Re::Kleen(c) => {
				print!("(");
				c.debug_print();
				print!(")*");
			},
			Re::Or(c1, c2) => {
				print!("(");
				c1.debug_print();
				print!("|");
				c2.debug_print();
				print!(")");
			},
			Re::And(c1, c2) => {
				print!("(");
				c1.debug_print();
				print!("");
				c2.debug_print();
				print!(")");
			},
			Re::OneOrMore(c) => {
				print!("(");
				c.debug_print();
				print!(")+");
			},
			Re::AnyChar => {
				print!(".");
			},
			Re::Repeat(c, a, b) => {
				print!("(");
				c.debug_print();
				print!("){{{a},{b}}}");
			},
		}
	}
}

fn first_non_ascii(string: &[u8]) -> Option<usize> {
	for i in 0..string.len() {
		if !string[i].is_ascii() {
			return Some(i)
		}
	}
	None
}

fn parse_or(string: &[u8], index: usize) -> Result<(Re, usize), ReError> {
	let (left, new_index) = parse_and(string, index)?;

	if new_index < string.len() {
		let sep = string[new_index] as char;
		if sep == '|' {
			let (right, end_index) = parse_or(string, new_index+1)?;
			Ok((Re::Or(Rc::from(left),Rc::from(right)), end_index))
		}
		else if sep == ')' {
			Ok((left, new_index+1))
		}
		else {
			Ok((left, new_index))
			// Err((
			// 	ReErrorKind::BadSplit,
			// 	ReErrorInfo{at: new_index, msg: String::from("Expected '('")}
			// ))
		}
	}
	else {
		Ok((left, new_index))
	}
}

fn parse_and(string: &[u8], index: usize) -> Result<(Re, usize), ReError> {
	let (left, after_left_id) = parse_postfix(string, index)?;

	// End of string, do not match more
	if after_left_id >= string.len() {
		return Ok((left, after_left_id));
	}
	
	// We have reached a closing parenthesis so we return the reg
	if string[after_left_id] == ')' as u8 {
		return Ok((left, after_left_id+1));
	}
	if string[after_left_id] == '|' as u8 {
		return Ok((left, after_left_id));
	}

	// Find the right reg
	let (right, after_right_id) = parse_or(string, after_left_id)?;

	Ok((Re::And(Rc::from(left), Rc::from(right)), after_right_id))
}

fn parse_number(string: &[u8], index: usize) -> Option<(usize, usize)> {
	let mut current_index = index;
	let mut current_string = String::new();

	while current_index < string.len() && string[current_index].is_ascii_digit() {
		current_string.push(string[current_index] as char);
		current_index += 1;
	}

	match current_string.parse::<usize>() {
		Ok(integer) => Some((integer, current_index)),
		Err(_) => None
	}
}

fn parse_postfix(string: &[u8], index: usize) -> Result<(Re, usize), ReError> {
	// match an underlying atom (either a single char or a sub regexp in parenthesis)
	let (reg, after_atom_id) = parse_atom(string, index)?;

	// Look for optionnal +, *, {a,b}
	if after_atom_id >= string.len() {
		return Ok((reg, after_atom_id));
	}

	if string[after_atom_id] == '*' as u8 {
		Ok((Re::Kleen(Rc::from(reg)), after_atom_id+1))
	}
	else if string[after_atom_id] == '+' as u8 {
		Ok((Re::OneOrMore(Rc::from(reg)), after_atom_id+1))
	}
	else if string[after_atom_id] == '{' as u8 {
		// try to parse the rest 
		match parse_number(string, after_atom_id+1) {
			None => Err((ReErrorKind::InvalidInt, ReErrorInfo{at: after_atom_id+1, msg: String::from("Expected a positive integer")})),
			Some((left, after_left_id)) => {
				if after_left_id >= string.len() || string[after_left_id] != ',' as u8 {
					return Err((ReErrorKind::OutOfRange, ReErrorInfo{at: after_atom_id, msg: String::from("Expected a ',' for postfix [reg]{a,b}")}));
				}
				match parse_number(string, after_left_id+1) {
					None => Err((ReErrorKind::InvalidInt, ReErrorInfo{at: after_left_id, msg: String::from("Expected a positive integer")})),
					Some((right, after_right_id)) => {
						if left > right {
							return Err((ReErrorKind::InvalidRange, ReErrorInfo{at: after_atom_id, msg: String::from("left number should be lower than or equal to the right one")}));
						}
						
						if after_right_id >= string.len() || string[after_right_id] != '}' as u8 {
							return Err((ReErrorKind::InvalidChar, ReErrorInfo{at: after_left_id, msg: String::from("Expected a '}'")}));
						}

						Ok((Re::Repeat(Rc::from(reg), left, right), after_right_id+1))
					}
				}
			}
		}
	}
	else {
		Ok((reg, after_atom_id))
	}
}

fn parse_atom(string: &[u8], index: usize) -> Result<(Re,usize), ReError> {
	// If we try to find an atom out of range, there must be an issue
	if index >= string.len() {
		return Err((ReErrorKind::OutOfRange, ReErrorInfo{at: index, msg: String::from("Expected an atom")}));
	}

	// for now, the parser only supports chars in the range [a-zA-Z0-9], we should use a separate function to take into account all other chars as well as \\, \(, ...
	if (string[index] >= 'a' as u8 && string[index] <= 'z' as u8)
		|| (string[index] >= 'A' as u8 && string[index] <= 'Z' as u8)
		|| (string[index] >= '0' as u8 && string[index] <= '9' as u8) {
		Ok((Re::Char(string[index] as char), index+1))
	}

	// match any char
	else if string[index] == '.' as u8 {
		Ok((Re::AnyChar, index+1))
	}

	// match a subexpression
	else if string[index] == '(' as u8 {
		return parse_or(string, index+1);
	}

	// Unsupported char
	else {
		return Err((ReErrorKind::InvalidChar, ReErrorInfo{at: index, msg: String::from("Expected a char in [a-zA-Z0-9]")}));
	}
}

#[cfg(test)]
mod tests {
    use crate::regexp::{parse_atom, Re, ReErrorKind};

    use super::{first_non_ascii, parse_number, parse_postfix};

	#[test]
	fn parse_integer() {
		let string = "abcdd12344,.88_".as_bytes();

		assert_eq!(parse_number(string, 4), None);
		assert_eq!(parse_number(string, 5), Some((12344, 10)));
		assert_eq!(parse_number(string, 12), Some((88, 14)));
		assert_eq!(parse_number(string, 15), None);
	}

	#[test]
	fn is_string_ascii() {
		assert_eq!(first_non_ascii("salut! Comment ca va?".as_bytes()), None);
		assert_eq!(first_non_ascii("salut! Comment ça va?".as_bytes()), Some(15));
	}

	#[test]
	fn atom_parsing() {
		let string = "a._?".as_bytes();

		match parse_atom(string, 0) {
			Ok((reg, index)) => {
				assert_eq!(index, 1);
				assert!(reg.is_equal(&Re::Char('a')))
			},
			Err(_) => assert!(false)
		}

		match parse_atom(string, 1) {
			Ok((reg, index)) => {
				assert_eq!(index, 2);
				assert!(reg.is_equal(&Re::AnyChar))
			},
			Err(_) => assert!(false)
		}

		match parse_atom(string, 3) {
			Ok(_) => assert!(false),
			Err(_) => ()
		}
	}

	#[test]
	fn reg_equality() {
		let reg1 = Re::AnyChar;
		let reg2 = Re::Kleen(std::rc::Rc::from(Re::AnyChar));

		assert!(reg1.is_equal(&reg1));
		assert!(!reg1.is_equal(&reg2));
	}

	#[test]
	fn postfix_parsing() {
		let string = "a{22,144}a{144,22}aa+*b".as_bytes();

		match parse_postfix(string, 0) {
			Err(_) => assert!(false),
			Ok((reg, index)) => {
				assert_eq!(index, 9);
				let r = Re::Repeat(std::rc::Rc::from(Re::Char('a')), 22, 144);
				assert!(reg.is_equal(&r));
			}
		}

		match parse_postfix(string, 9) {
			Err((err_kind, _)) => {
				assert!(matches!(err_kind, ReErrorKind::InvalidRange));
			},
			Ok(_) => assert!(false)
		}
	}

	#[test]
	fn and_parsing() {
		
	}
}