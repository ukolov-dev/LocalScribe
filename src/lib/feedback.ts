export type FeedbackState = {
  error: string;
  notice: string;
};

export function copySuccessFeedback(message: string): FeedbackState {
  return {
    error: "",
    notice: message
  };
}

export function copyFailureFeedback(message: string): FeedbackState {
  return {
    error: message,
    notice: ""
  };
}
