use witgen::witgen;
use witgen_dep_dep::InnerType;

#[witgen]
pub struct TestDep {
    pub inner: InnerType,
}
