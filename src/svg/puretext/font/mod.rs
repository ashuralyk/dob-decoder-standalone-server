use lazy_static::lazy_static;
use ttf_parser::Face;

pub mod path;

lazy_static! {
    pub static ref TURRETROAD_400_TTF: Face<'static> =
        Face::parse(include_bytes!("turretroad-400.ttf"), 0).expect("parse turretroad-400.ttf");
    pub static ref TURRETROAD_700_TTF: Face<'static> =
        Face::parse(include_bytes!("turretroad-700.ttf"), 0).expect("parse turretroad-700.ttf");
}
