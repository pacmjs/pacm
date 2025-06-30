import e from "express";
import chalk from "chalk";

const app = e();

app.get("/", (req, res) => {
  res.json({
    message: "Hello, PACM powered server!"
  });
});

app.listen(3000, () => {
  console.log(chalk.green("Server is running on http://localhost:3000"));
});