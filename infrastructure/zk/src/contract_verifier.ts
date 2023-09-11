import { Command } from 'commander';
import * as utils from './utils';

export async function contractVerifier() {
    await utils.spawn(`cargo run --bin micro_contract_verifier --release`);
}

export const command = new Command('contract_verifier')
    .description('start micro contract verifier')
    .action(contractVerifier);
