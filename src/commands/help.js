function displayHelp() {
    console.log(`
      Usage: pacm [command]
  
      Commands:
        help      Display this help message
        version   Show the version of pacm
    `);
}

export default function help() {
    displayHelp();
};