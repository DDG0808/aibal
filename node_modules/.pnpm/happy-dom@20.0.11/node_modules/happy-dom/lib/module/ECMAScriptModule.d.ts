import BrowserWindow from '../window/BrowserWindow.js';
import { URL } from 'url';
import IModule from './IModule.js';
import * as PropertySymbol from '../PropertySymbol.js';
import Location from '../location/Location.js';
/**
 * ECMAScript module.
 */
export default class ECMAScriptModule implements IModule {
    #private;
    readonly url: URL;
    readonly [PropertySymbol.window]: BrowserWindow;
    /**
     * Constructor.
     *
     * @param window Window.
     * @param url Module URL.
     * @param source Source code.
     * @param [sourceURL] Source URL.
     */
    constructor(window: BrowserWindow, url: URL | Location, source: string, sourceURL?: string | null);
    /**
     * Compiles and evaluates the module.
     *
     * @returns Module exports.
     */
    evaluate(): Promise<{
        [key: string]: any;
    }>;
    /**
     * Compiles and preloads the module and its imports.
     *
     * @returns Promise.
     */
    preload(): Promise<void>;
}
//# sourceMappingURL=ECMAScriptModule.d.ts.map