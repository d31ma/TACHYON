// @ts-check
/**
 * @typedef {{
 *   requestId: string
 *   ipAddress: string
 *   protocol: string
 *   host: string
 *   bearer?: { token: string, verified: boolean }
 * }} YonContext
 */

export default {
    /**
     * @param {Request} request
     * @param {YonContext} context
     * @returns {Promise<Response | void>}
     */
    async before(request, context) {
        const pathname = new URL(request.url).pathname;
        console.info(`[middleware] ${request.method} ${pathname} requestId=${context.requestId}`);
    },
    /**
     * @param {Request} request
     * @param {Response} response
     * @param {YonContext} context
     * @returns {Promise<Response>}
     */
    async after(request, response, context) {
        return response;
    }
};
