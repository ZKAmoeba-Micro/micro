import { Command } from 'commander';
import * as utils from './utils';
import fs from 'fs';

// Make sure that the volumes exists before starting the containers.
function createVolumes() {
    fs.mkdirSync(`${process.env.MICRO_HOME}/volumes/lotus`, { recursive: true });
    fs.mkdirSync(`${process.env.MICRO_HOME}/volumes/postgres`, { recursive: true });
}

export async function up() {
    createVolumes();
    await utils.spawn('docker-compose up -d lotus postgres');
}

export const command = new Command('up').description('start development containers').action(up);
