import logger from "../lib/logger.js";
import path from "node:path";
import process from "node:process";
import { exec } from "node:child_process";

export default function self_update(args) {
    if (args.length > 0) {
        logger.logError({
            message: "This command does not take any arguments.",
            exit: true,
            errorType: " PACM_ARG_VALIDATION_ERROR ",
        });
    }

    let os = process.platform;
    let arch = process.arch;
    let url = "";
    let filePath = "";
    let fileName = "";

    if (os === "win32") {
        url = "https://github.com/pacmjs/pacm/releases/latest/download/pacm.exe";
        filePath = path.join("C:", "Program Files", "pacm");
        fileName = "pacm.exe";
    } else if (os === "linux") {
        url = "https://github.com/pacmjs/pacm/releases/latest/download/pacm";
        filePath = "/usr/local/bin";
        fileName = "pacm";
    } else if (os === "darwin") {
        url = "https://github.com/pacmjs/pacm/releases/latest/download/pacm";
        filePath = "/usr/local/bin";
        fileName = "pacm";
    } else {
        logger.logError({
            message: "Unsupported operating system and/or architecture.",
            exit: true,
            errorType: " PACM_UNSUPPORTED_OS_ERROR ",
        });
    }

    exec(`curl -L ${url} -o ${fileName}`, (error, stdout, stderr) => {
        if (error) {
            logger.logError({
                message: "An error occurred while downloading the latest pacm executable.",
                exit: true,
                errorType: " PACM_SELF_UPDATE_ERROR ",
            });
        }

        console.log(stdout);

        if (os === "win32") {
            exec(`move ${fileName} ${filePath}`, (error, stdout, stderr) => {
                if (error) {
                    logger.logError({
                        message: "An error occurred while moving the latest pacm executable.",
                        exit: true,
                        errorType: " PACM_SELF_UPDATE_ERROR ",
                    });
                }
            });
        } else {
            exec(`sudo mv ${fileName} ${filePath}`, (error, stdout, stderr) => {
                if (error) {
                    logger.logError({
                        message: "An error occurred while moving the latest pacm executable.",
                        exit: true,
                        errorType: " PACM_SELF_UPDATE_ERROR ",
                    });
                }
            });
        }
    });

    logger.logSuccess({
        message: "Successfully updated pacm to the latest version.",
        successType: " PACM_SELF_UPDATE_SUCCESS ",
    });

    process.exit(0);
}