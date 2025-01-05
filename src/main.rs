mod regexp;
use regexp::Re;

fn main() {
	let regexp = Re::parse_regexp("(a|b)*a").unwrap();
	regexp.debug_print();
	println!("");
}