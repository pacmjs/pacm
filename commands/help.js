import chalk from "chalk";

function displayHelp() {
  console.log(`
      Usage: ${chalk.blueBright("pacm")} [command]
  
      Commands:
        help        Display this help message
        version     Show the version of pacm
        init        Initialize a new pacm project
        install     Install dependencies
        info        Display information about a package
        add         Add a new dependency
        remove      Remove a dependency
        list        List all dependencies
        update      Update a dependency
        search      Search for a package
        self-update Update pacm to the latest version
        clean       Clean the cache
        run         Run a script
        publish     Publish a package

      Options:
        -h, --help     Display this help message
        -v, --version  Show the version of pacm
        -f, --force    Force the operation
        -D, --dev      Add a dependency as a devDependency
      
      Examples:
        ${chalk.blueBright("pacm")} install
        ${chalk.blueBright("pacm")} info lodash
        ${chalk.blueBright("pacm")} add lodash
        ${chalk.blueBright("pacm")} remove lodash
        ${chalk.blueBright("pacm")} list
        ${chalk.blueBright("pacm")} update lodash
        ${chalk.blueBright("pacm")} search lodash
        ${chalk.blueBright("pacm")} self-update
        ${chalk.blueBright("pacm")} clean
        ${chalk.blueBright("pacm")} run start
        ${chalk.blueBright("pacm")} publish
      
      For more information, visit:
      - ${chalk.cyan("https://pacmjs.buzzr.land")}
      - ${chalk.green("https://github.com/pacmjs/pacm")}
    `);
}

export default function help() {
  displayHelp();
}
