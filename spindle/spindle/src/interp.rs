use crate::vm::VmProgram;

pub struct Interp<'vm> {
    program: &'vm VmProgram<'vm>,
}

impl<'vm> Interp<'vm> {
    pub fn new(program: &'vm VmProgram<'vm>) -> Self {
        Interp { program }
    }

}
