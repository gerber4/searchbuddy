const {resolve} = require("path");

module.exports = {
    context: resolve(__dirname, "../../src"),
    entry: {
        content: "./content.ts",
        searchbuddy: "./searchbuddy/searchbuddy.tsx",
        backgroundPage: "./backgroundPage.ts",
    },
    output: {
        filename: "js/[name].js",
        path: resolve(__dirname, "../../dist"),
    },
    performance: {
        hints: false,
    },
    resolve: {
        extensions: [".js", ".jsx", ".ts", ".tsx", ".css"],
    },
    module: {
        rules: [
            {
                test: [/\.tsx?$/],
                use: ["ts-loader"],
                exclude: /node_modules/,
            },
            {
                test: [/\.jsx?$/],
                use: ["babel-loader"],
                exclude: /node_modules/,
            },
            {
                test: /\.css$/,
                use: ["style-loader", "css-loader"],
            },
        ],
    }
};
