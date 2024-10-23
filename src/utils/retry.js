export async function retryOnECONNRESET(fn, ...args) {
  for (let attempt = 1; attempt <= 3; attempt++) {
    try {
      return await fn(...args);
    } catch (error) {
      if (error.code === "ECONNRESET") {
        console.warn(`Warning: ${args[0]} error ECONNRESET, retry ${attempt}`);
        if (attempt === 3) throw error;
      } else {
        throw error;
      }
    }
  }
}
