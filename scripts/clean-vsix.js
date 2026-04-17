const { readdirSync, unlinkSync } = require("fs");

readdirSync(".")
  .filter((n) => n.endsWith(".vsix"))
  .forEach((name) => {
    unlinkSync(name);
    console.log("Removed " + name);
  });
