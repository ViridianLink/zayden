mod custom_msg;
mod random;

use custom_msg::CustomMsg;
use random::Random;

use crate::RegistryBuilder;

pub fn register(builder: &mut RegistryBuilder) {
    builder.add_command(Random).add_command(CustomMsg);
}
