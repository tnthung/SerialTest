

macro_rules! block {
  ($xs:block) => {
      loop { $xs break; }
  };
}


pub(crate) use block;
