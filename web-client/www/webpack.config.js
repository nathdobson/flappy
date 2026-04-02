const CopyWebpackPlugin = require("copy-webpack-plugin");
const path = require('path');

module.exports = {
  entry: "./bootstrap.js",
  experiments: {
          asyncWebAssembly: true
  },
  output: {
    path: path.resolve(__dirname, "dist"),
    filename: "bootstrap.js",
    library: "index"
  },
  mode: "development",
  plugins: [
    new CopyWebpackPlugin([
        'index.html',
        'index.css',
        'favicon.png',
    ])
  ],
};
