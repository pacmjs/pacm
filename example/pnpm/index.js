import axios from "axios";

const main = async () => {
  const response = await axios.get(
    "https://jsonplaceholder.typicode.com/todos/1",
  );
  console.log(response.data);
};

main();
