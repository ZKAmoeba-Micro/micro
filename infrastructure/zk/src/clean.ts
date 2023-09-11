import { Command } from 'commander';
import * as fs from 'fs';
import * as path from 'path';
import { confirmAction } from './utils';

export function clean(path: string) {
    if (fs.existsSync(path)) {
        fs.rmSync(path, { recursive: true });
        console.log(`Successfully removed ${path}`);
    }
}

export const command = new Command('clean')
    .option('--config [environment]')
    .option('--database')
    .option('--contracts')
    .option('--artifacts')
    .option('--all')
    .description('removes generated files')
    .action(async (cmd) => {
        if (!cmd.contracts && !cmd.config && !cmd.database && !cmd.backups && !cmd.artifacts) {
            cmd.all = true; // default is all
        }
        await confirmAction();

        if (cmd.all || cmd.config) {
            const env = process.env.MICRO_ENV;
            clean(`etc/env/${env}.env`);
            clean('etc/env/.init.env');
        }

        if (cmd.all || cmd.artifacts) {
            clean(`artifacts`);
        }

        if (cmd.all || cmd.database) {
            const dbPath = process.env.DATABASE_PATH!;
            clean(path.dirname(dbPath));
        }

        if (cmd.all || cmd.contracts) {
            clean('contracts/ethereum/artifacts');
            clean('contracts/ethereum/cache');
            clean('contracts/ethereum/typechain');
            clean('contracts/micro/artifacts-zk');
            clean('contracts/micro/cache-zk');
            clean('contracts/micro/typechain');
        }
    });
