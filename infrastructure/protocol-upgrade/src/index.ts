import { program } from 'commander';

import { command as publish } from './l2upgrade/system-contracts';
import { command as manager } from './protocol-upgrade-manager';
import { command as customUpgrade } from './custom-upgrade';
import { command as l1Upgrade } from './l1upgrade/facets';
import { command as l2Upgrade } from './l2upgrade/transactions';
import { command as transactions } from './transaction';
import { command as crypto } from './crypto/crypto';

const COMMANDS = [publish, manager, customUpgrade, l1Upgrade, transactions, crypto, l2Upgrade];

async function main() {
    const MICRO_HOME = process.env.MICRO_HOME;

    if (!MICRO_HOME) {
        throw new Error('Please set $MICRO_HOME to the root of micro repo!');
    } else {
        process.chdir(MICRO_HOME);
    }

    program.version('0.1.0').name('zk').description('micro protocol upgrade tools');

    for (const command of COMMANDS) {
        program.addCommand(command);
    }
    await program.parseAsync(process.argv);
}

main().catch((err: Error) => {
    console.error('Error:', err.message || err);
    process.exitCode = 1;
});
