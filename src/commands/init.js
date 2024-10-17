import fs from 'node:fs';
import path from 'node:path';
import prompts from 'prompts';

export default async function init() {
    const args = process.argv.slice(2);
    const useDefaults = args.includes('-y') || args.includes('--defaults');

    const questions = [
        {
            type: 'text',
            name: 'name',
            message: 'Package name:',
            initial: 'my-package',
            skip: useDefaults
        },
        {
            type: 'text',
            name: 'version',
            message: 'Version:',
            initial: '1.0.0',
            skip: useDefaults
        },
        {
            type: 'text',
            name: 'description',
            message: 'Description:',
            skip: useDefaults
        },
        {
            type: 'text',
            name: 'entry',
            message: 'Entry point:',
            initial: 'index.js',
            skip: useDefaults
        },
        {
            type: 'text',
            name: 'author',
            message: 'Author:',
            skip: useDefaults
        },
        {
            type: 'text',
            name: 'license',
            message: 'License:',
            initial: 'ISC',
            skip: useDefaults
        }
    ];

    const response = useDefaults ? {
        name: 'my-package',
        version: '1.0.0',
        description: '',
        entry: 'index.js',
        author: '',
        license: 'ISC'
    } : await prompts(questions);

    const packageJson = {
        name: response.name,
        version: response.version,
        description: response.description,
        main: response.entry,
        scripts: {
            test: 'echo "Error: no test specified" && exit 1'
        },
        author: response.author,
        license: response.license
    };

    fs.writeFileSync(
        path.join(process.cwd(), 'package.json'),
        JSON.stringify(packageJson, null, 2)
    );

    console.log('package.json has been created successfully.');
}