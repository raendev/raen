"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getBinary = exports.GithubUrl = void 0;
const _1 = require(".");
const path_1 = require("path");
const os = require("os");
const { version } = require("../package.json");
const NAME = "raen";
function getPlatform() {
    const type = os.type();
    const arch = os.arch();
    let typeDict = {
        "Darwin": "apple-darwin",
        "Linux": "unknown-linux-gnu",
        "Windows_NT": "pc-windows-msvc"
    };
    let archDict = {
        "x64": "x86_64",
        "arm64": "aarch64"
    };
    //@ts-ignore 
    let rust_type = typeDict[type];
    //@ts-ignore 
    let rust_arch = archDict[arch];
    if (rust_type && rust_arch) {
        return [rust_type, rust_arch];
    }
    throw new Error(`Unsupported platform: ${type} ${arch}`);
}
function GithubUrl() {
    const [platform, arch] = getPlatform();
    return `https://github.com/raendev/${NAME}/releases/download/v${version}/${NAME}-v${version}-${arch}-${platform}.tar.gz`;
}
exports.GithubUrl = GithubUrl;
function getBinary(name = NAME) {
    if (!process.env["RAEN_BIN_PATH"]) {
        process.env["RAEN_BINARY_PATH"] = (0, path_1.join)(os.homedir(), `.${NAME}`, NAME);
    }
    // Will use version after publishing to AWS
    // const version = require("./package.json").version;
    const fromEnv = process.env["RAEN_ARTIFACT_URL"];
    const urls = [GithubUrl()];
    if (fromEnv) {
        urls.unshift(fromEnv);
    }
    return _1.Binary.create(name, urls);
}
exports.getBinary = getBinary;
