const {existsSync, readFileSync} = require('fs')
const {join} = require('path')

const {platform, arch} = process

let nativeBinding = null
let localFileExisted = false
let loadError = null

function isMusl() {
    // For Node 10
    if (!process.report || typeof process.report.getReport !== 'function') {
        try {
            return readFileSync('/usr/bin/ldd', 'utf8').includes('musl')
        } catch (e) {
            return true
        }
    } else {
        const {glibcVersionRuntime} = process.report.getReport().header
        return !glibcVersionRuntime
    }
}

switch (platform) {
    case 'darwin':
        switch (arch) {
            case 'x64':
                nativeBinding = require('./index.darwin-x64.node')
                break
            case 'arm64':
                nativeBinding = require('./index.darwin-arm64.node')
                break
            default:
                throw new Error(`Unsupported architecture on macOS: ${arch}`)
        }
        break
    default:
        throw new Error(`Unsupported OS: ${platform}, architecture: ${arch}`)
}

if (!nativeBinding) throw new Error(`Failed to load native binding`)

const {RedisClient} = nativeBinding

module.exports.RedisClient = RedisClient
