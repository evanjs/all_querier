let provider_name = variable::get("project-name");

if provider_name.is_empty() {
    provider_name = variable::prompt("What will this provider be named?").to_snake_case();
}

if provider_name.starts_with("allq-") {
    provider_name.replace("allq-", "");
}

if provider_name.is_empty() {
    abort("Provider name must not be empty.");
}

variable::set("provider_name", provider_name);
variable::set("project-name", "allq-" + provider_name);

print(`Creating a new provider named "${variable::get("provider_name")}"`);
print(`Your provider crate will be named "${variable::get("project-name")}".`);
print("");
print("Note that the MIT license is used by default");
print("You can change this manually if you'd like to.");
print("");
//print("!!! IMPORTANT !!!");
//print("You must run cargo generate with --force, or it will rename your project to");
//print("something that is non-standard for allq providers and this will fail.");
//print("");
//print("If you see a message after this about renaming your project, please abort and");
//print("try again with --force.");
//print("");