// @ts-check

import LanguageRepository from '../repositories/language-repository.js';
import FyloMachineRepository from '../repositories/fylo-machine-repository.js';

export default class LanguageService {
    /**
     * @param {{ repository?: LanguageRepository, fyloRepository?: FyloMachineRepository }} [options]
     */
    constructor(options = {}) {
        this.repository = options.repository ?? new LanguageRepository();
        this.fyloRepository = options.fyloRepository ?? new FyloMachineRepository();
    }

    /**
     * @param {Record<string, any>} request
     * @returns {Record<string, unknown>}
     */
    diagnostics(request) {
        return {
            message: 'Hello from Yon on Bun!',
            runtime: this.repository.runtime(),
            context: request.context,
            frontend: 'Tac',
            backend: 'Yon',
        };
    }

    /**
     * @param {Record<string, any>} request
     * @returns {Promise<Record<string, unknown>>}
     */
    async fyloDemo(request) {
        const requestId = String(request.context?.requestId ?? 'unknown');
        return await this.fyloRepository.demonstrateAll(requestId);
    }

    /**
     * @param {Record<string, any>} request
     * @returns {Record<string, unknown>}
     */
    createEcho(request) {
        return {
            message: 'Hello from Yon language service!',
            snapshot: this.repository.snapshot({ requestId: request.context?.requestId }),
        };
    }

    /**
     * @param {Record<string, any>} request
     * @returns {Record<string, unknown>}
     */
    updateEcho(request) {
        return {
            message: 'Hello from Yon on Bun!',
            method: 'PUT',
            context: request.context,
            snapshot: this.repository.snapshot({ requestId: request.context?.requestId }),
        };
    }
}
