import { Binary } from ".";
import { join } from "path";
import * as os from "os";
const {version} = require("../package.json");

const NAME = "raen"

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
 let rust_type: string? = typeDict[type];
 //@ts-ignore 
 let rust_arch: string? = archDict[arch];

  if (rust_type && rust_arch) {
    return [rust_type, rust_arch];
  }
  throw new Error(`Unsupported platform: ${type} ${arch}`);
}

export function AWSUrl(): string {
  const [platform, arch] = getPlatform();
  return `https://github.com/AhaLabs/${NAME}/releases/download/v${version}/${NAME}-v${version}-${arch}-${platform}.tar.gz`;
}

export function getBinary(name: string = NAME): Promise<Binary> {
  if (!process.env["RAEN_BIN_PATH"]) {
    process.env["RAEN_BINARY_PATH"] = join(
      os.homedir(),
      `.${NAME}`,
      NAME
    );
  }

  // Will use version after publishing to AWS
  // const version = require("./package.json").version;
  const fromEnv = process.env["RAEN_ARTIFACT_URL"];
  const urls = [AWSUrl()];
  if (fromEnv) {
    urls.unshift(fromEnv);
  }

  return Binary.create(name, urls);
}
