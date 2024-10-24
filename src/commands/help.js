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
        link      Link a package
        unlink    Unlink a package
        publish   Publish a package to the NPM registry
    `);
}

export default function help() {
  displayHelp();
}
