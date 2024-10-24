function displayHelp() {
  console.log(`
      Usage: pacm [command]
  
      Commands:
        help      Display this help message
        version   Show the version of pacm
        init      Initialize a new pacm project
        install   Install dependencies
        add       Add a new dependency
        remove    Remove a dependency
        list      List all dependencies
        update    Update a dependency
        search    Search for a package
        clean     Clean the cache
        run       Run a script

      Options:
        -h, --help     Display this help message
        -v, --version  Show the version of pacm
        -f, --force    Force the operation
        -D, --dev      Add a dependency as a devDependency
    `);
}

export default function help() {
  displayHelp();
}
