import chalk from "chalk";
import boxen from "boxen";
import fetch from "node-fetch";

export default function UpdateCheck() {
  return new Promise((resolve) => {
    const repo = "pacmjs/pacm";
    const version = "v1.0.0";
    const url = `https://api.github.com/repos/${repo}/releases/latest`;

    fetch(url)
      .then((res) => res.json())
      .then((json) => {
        if (json.message === "Not Found") {
          resolve();
          return;
        }

        if (json.tag_name !== version) {
          const updateMessage = `
${chalk.bgYellow.whiteBright(" Update available! \n")}
${chalk.red(`${version}`)} => ${chalk.green(`${json.tag_name}`)}
${chalk.blue(`${json.html_url}`)}
Run ${chalk.green(`pacm self-update`)} to update.
                `;

          const boxedMessage = boxen(updateMessage, {
            padding: 0.5,
            margin: 1,
            borderColor: "white",
            borderStyle: "round",
            align: "center",
          });

          console.log(boxedMessage);
        }
        resolve();
      })
      .catch(() => {});
  });
}
