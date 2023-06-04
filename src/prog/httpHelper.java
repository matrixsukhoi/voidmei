package prog;

import java.io.BufferedInputStream;
import java.io.BufferedReader;
import java.io.BufferedWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.io.OutputStreamWriter;
import java.net.InetSocketAddress;
import java.net.Socket;
import java.net.SocketAddress;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.Executors;

public class httpHelper {
	public CompletableFuture<Boolean> completableFuture0 = new CompletableFuture<Boolean>();
	public CompletableFuture<Boolean> completableFuture1 = new CompletableFuture<Boolean>();
	public String state_request = "GET " + "/state" + " HTTP/1.1\n" + "Host: " + "127.0.0.1" + "\n"
			+ "Cache-Control:no-cache\n" + app.httpHeader + "\n";
	public String indic_request = "GET " + "/indicators" + " HTTP/1.1\n" + "Host: " + "127.0.0.1" + "\n"
			+ "Cache-Control:no-cache\n" + app.httpHeader + "\n";
	public String fmcm_request = "GET " + "/editor/fm_commands?cmd=getFmProperties" + " HTTP/1.1\n" + "Host: "
			+ "127.0.0.1" + "\n" + "Cache-Control:no-cache\n" + app.httpHeader + "\n";
	public String strState;
	public String strIndic;
	public static final String nstring="";

	public String sendGet(String host, int port, String path) throws IOException {

		String result = nstring;
		Socket socket = new Socket();
		SocketAddress dest = new InetSocketAddress(host, port);
		socket.connect(dest);
		OutputStreamWriter streamWriter = new OutputStreamWriter(socket.getOutputStream());
		BufferedWriter bufferedWriter = new BufferedWriter(streamWriter);

		bufferedWriter.write("GET " + path + " HTTP/1.1\r\n");
		bufferedWriter.write("Host: " + host + "\r\n");
		bufferedWriter.write("Cache-Control:no-cache");
		bufferedWriter.write(app.httpHeader);
		bufferedWriter.write("\r\n");
		bufferedWriter.flush();

		BufferedInputStream streamReader = new BufferedInputStream(socket.getInputStream());

		BufferedReader bufferedReader = new BufferedReader(new InputStreamReader(streamReader, "utf-8"));

		String line = null;

		bufferedReader.ready();
		bufferedReader.readLine();

		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();

		StringBuilder contentBuf = new StringBuilder();
		while ((line = bufferedReader.readLine()) != null) {
			contentBuf.append(line);
		}
		result = contentBuf.toString();
		// app.debugPrint(result);
		bufferedReader.close();
		bufferedWriter.close();
		socket.close();

		return result;
	}


	public String sendGetFast(String req_string, SocketAddress dest) throws IOException {
		String result = null;
		Socket socket = new Socket();
		// socket.
		socket.connect(dest);
		OutputStreamWriter streamWriter = new OutputStreamWriter(socket.getOutputStream());
		BufferedWriter bufferedWriter = new BufferedWriter(streamWriter);

		BufferedInputStream streamReader = new BufferedInputStream(socket.getInputStream());

		BufferedReader bufferedReader = new BufferedReader(new InputStreamReader(streamReader, "utf-8"));

		bufferedWriter.write(req_string);
		// bufferedWriter.write("\r\n");
		bufferedWriter.flush();

		// BufferedInputStream streamReader = new
		// BufferedInputStream(socket.getInputStream());
		//
		// BufferedReader bufferedReader = new BufferedReader(new
		// InputStreamReader(streamReader, "utf-8"));

		String line = null;

		// if (bufferedReader.ready()) {
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();

		StringBuilder contentBuf = new StringBuilder();
		while ((line = bufferedReader.readLine()) != null) {
			contentBuf.append(line);
		}
		result = contentBuf.toString();
		// }
		// else{
		// result = nastring;
		// }
		// app.debugPrint(result);
		bufferedReader.close();
		bufferedWriter.close();
		socket.close();
		// socket = null;
		// bufferedReader = null;
		// bufferedWriter = null;
		//
		// streamWriter = null;
		// contentBuf = null;
		return result;
	}
	// public Future<String> sendGetAsync(CompletableFuture<String>
	// completableFuture, String req_string, SocketAddress dest) throws
	// InterruptedException {
	//// CompletableFuture<String> completableFuture = new
	// CompletableFuture<String>();
	// Executors.newCachedThreadPool().submit(() -> {
	//// System.out.println("submit\n");
	// s = sendGetFast(req_string, dest);
	//// System.out.println("complete\n");
	// completableFuture.complete(s);
	// return null;
	// });
	//
	// return completableFuture;
	// }
	//

	public void getReqResult(SocketAddress req_addr) {
		try {

			// Executors.newCachedThreadPool().submit(() -> {
			// s = sendGetFast(state_request, req_addr);
			// completableFuture0.complete(true);
			// return null;
			// });
			// Executors.newCachedThreadPool().submit(() -> {
			// s1 = sendGetFast(indic_request, req_addr);
			// completableFuture1.complete(true);
			// return null;
			// });
			//// s1 = sendGetFast(indic_request, req_addr);
			// try {
			// completableFuture0.get(4, TimeUnit.MILLISECONDS);
			// } catch (TimeoutException e) {
			//
			// System.out.println("s Time Out\n");
			//
			//// completableFuture0.cancel(true);
			// s = nullstring;
			// }
			//
			// try {
			// completableFuture1.get(4, TimeUnit.MILLISECONDS);
			// } catch (TimeoutException e) {
			// System.out.println("s1 Time Out\n");
			//// completableFuture1.cancel(true);
			// s1 = nullstring;
			// }

			Executors.newCachedThreadPool().submit(() -> {
				strState = sendGetFast(state_request, req_addr);
				completableFuture0.complete(true);
				return null;
			});

			strIndic = sendGetFast(indic_request, req_addr);
			completableFuture0.get();
			// try {
			// completableFuture0.get(4, TimeUnit.MILLISECONDS);
			// } catch (TimeoutException e) {
			//
			// System.out.println("s Time Out\n");
			//
			//// completableFuture0.cancel(true);
			// s = nullstring;
			// }

			// intv = freq;

		} catch (InterruptedException e) {
			// TODO Auto-generated catch block
			e.printStackTrace();
			System.out.println("InterruptedException\n");
			strState = nstring;
			strIndic = nstring;
			 
		} catch (ExecutionException e) {
			// TODO Auto-generated catch block
//			System.out.println("ExecutionException\n");
//			e.printStackTrace();
			strState = nstring;
			strIndic = nstring;
		} catch (IOException e1) {
			// TODO Auto-generated catch block
//			System.out.println("IOException\n");
//			e1.printStackTrace();
			strState = nstring;
			strIndic = nstring;
			
		} 
	}

}
