//if variable::get("project-name").contains("-") {
//    abort("Please run cargo generate with --force, so that we can set the crate name to\n\
//        something starting with `allq-`.");
//}

variable::set("provider_struct", variable::get("provider_name").to_pascal_case() + "Provider");
variable::set("provider_module", variable::get("provider_name").to_snake_case());