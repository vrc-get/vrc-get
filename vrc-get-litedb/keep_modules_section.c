// dummy to force the linker to include the "module" section in the final binary
__attribute__((retain,used,section("modules")))
static const char keep_modules_section_dummy[0];
