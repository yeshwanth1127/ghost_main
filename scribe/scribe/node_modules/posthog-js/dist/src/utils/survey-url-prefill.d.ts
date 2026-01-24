import { Survey } from '../posthog-surveys-types';
/**
 * Extracted URL prefill parameters by question index
 */
export interface PrefillParams {
    [questionIndex: number]: string[];
}
/**
 * Extract prefill parameters from URL search string
 * Format: ?q0=1&q1=8&q2=0&q2=2&auto_submit=true
 * NOTE: Manual parsing for IE11/op_mini compatibility (no URLSearchParams)
 */
export declare function extractPrefillParamsFromUrl(searchString: string): {
    params: PrefillParams;
    autoSubmit: boolean;
};
/**
 * Convert URL prefill values to SDK response format
 */
export declare function convertPrefillToResponses(survey: Survey, prefillParams: PrefillParams): Record<string, any>;
/**
 * Check if all REQUIRED questions that support prefill are filled
 */
export declare function allRequiredQuestionsFilled(survey: Survey, responses: Record<string, any>): boolean;
