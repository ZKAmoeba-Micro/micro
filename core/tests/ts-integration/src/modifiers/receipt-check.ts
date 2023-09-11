import { MatcherModifier, MatcherMessage } from '.';
import * as micro from 'micro-web3';

/**
 * Creates a custom checker for the transaction receipt.
 *
 * @param checkFn Function to check the receipt. Must return `true` if check passed, and `false` otherwise.
 * @param failMessage Message to be displayed if check wasn't passed.
 * @returns Matcher modifier object.
 */
export function checkReceipt(
    checkFn: (receipt: micro.types.TransactionReceipt) => boolean,
    failMessage: string
): ShouldCheckReceipt {
    return new ShouldCheckReceipt(checkFn, failMessage);
}

/**
 * Generic modifier capable of checking any data available in receipt.
 * Applied provided closure to the receipt.
 */
class ShouldCheckReceipt extends MatcherModifier {
    checkFn: (receipt: micro.types.TransactionReceipt) => boolean;
    failMessage: string;

    constructor(checkFn: (receipt: micro.types.TransactionReceipt) => boolean, failMessage: string) {
        super();
        this.checkFn = checkFn;
        this.failMessage = failMessage;
    }

    async check(receipt: micro.types.TransactionReceipt): Promise<MatcherMessage | null> {
        if (!this.checkFn(receipt)) {
            return {
                pass: false,
                message: () => this.failMessage
            };
        }

        return null;
    }
}
