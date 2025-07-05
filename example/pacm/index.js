import e from "express";

const app = e();

app.get("/", (req, res) => {
    res.send("Hello from PACM!");
});

app.listen(3000, () => {
    console.log("PACM is running on port 3000");
});