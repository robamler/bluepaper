const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const dist = path.resolve(__dirname, "dist");

module.exports = {
  mode: "production",
  entry: {
    index: "./js/index.js"
  },
  experiments: {
    asyncWebAssembly: true,
  },
  output: {
    path: dist,
    filename: "[name].js",
    libraryTarget: 'var',
    library: 'jsmodule'
  },
  devServer: {
    static: "./static",
    https: true,
  },
  plugins: [
    new CopyPlugin({
      patterns: [
        { from: 'static' },
      ],
    }),

    new WasmPackPlugin({
      forceMode: "production",
      crateDirectory: __dirname,
    }),
  ]
};
