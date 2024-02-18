package com.voidmei;

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
			+ "Cache-Control:no-cache\n" + App.httpHeader + "\n";
	public String indic_request = "GET " + "/indicators" + " HTTP/1.1\n" + "Host: " + "127.0.0.1" + "\n"
			+ "Cache-Control:no-cache\n" + App.httpHeader + "\n";
	public String fmcm_request = "GET " + "/editor/fm_commands?cmd=getFmProperties" + " HTTP/1.1\n" + "Host: "
			+ "127.0.0.1" + "\n" + "Cache-Control:no-cache\n" + App.httpHeader + "\n";
	public String setAltReq = "GET " + "/editor/fm_commands?cmd=setAlt&value=%d"+ " HTTP/1.1\n" + "Host: "
			+ "127.0.0.1" + "\n" + "Cache-Control:no-cache\n" + App.httpHeader + "\n";
	public String setVelReq = "GET " + "/editor/fm_commands?cmd=setVelocity&value=%.0f"+ " HTTP/1.1\n" + "Host: "
			+ "127.0.0.1" + "\n" + "Cache-Control:no-cache\n" + App.httpHeader + "\n";
	public String strState;
	public String strIndic;
	
	public StringBuilder strBState = new StringBuilder();
	public StringBuilder strBIndic = new StringBuilder();
	
	public static final String nstring="";
	public void fmCmdSetAlt(int alt, SocketAddress dest) throws IOException{
		String tmp_req = String.format(setAltReq, alt);
		Socket socket = new Socket();
		// socket.
		socket.connect(dest);
		OutputStreamWriter streamWriter = new OutputStreamWriter(socket.getOutputStream());
		BufferedWriter bufferedWriter = new BufferedWriter(streamWriter);

		BufferedInputStream streamReader = new BufferedInputStream(socket.getInputStream());

		BufferedReader bufferedReader = new BufferedReader(new InputStreamReader(streamReader, "utf-8"));

		bufferedWriter.write(tmp_req);
		bufferedWriter.flush();
		bufferedReader.close();
		socket.close();
	}
	public void fmCmdSetSpd(double spd, SocketAddress dest) throws IOException{
		String tmp_req = String.format(setVelReq, spd);
		Socket socket = new Socket();
		// socket.
		socket.connect(dest);
		OutputStreamWriter streamWriter = new OutputStreamWriter(socket.getOutputStream());
		BufferedWriter bufferedWriter = new BufferedWriter(streamWriter);

		BufferedInputStream streamReader = new BufferedInputStream(socket.getInputStream());

		BufferedReader bufferedReader = new BufferedReader(new InputStreamReader(streamReader, "utf-8"));

		bufferedWriter.write(tmp_req);
		bufferedWriter.flush();
		bufferedReader.close();
		socket.close();
	}

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
		bufferedWriter.write(App.httpHeader);
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
		// App.debugPrint(result);
		bufferedReader.close();
		bufferedWriter.close();
		socket.close();

		return result;
	}
	
	public static final int buf_len = 8192;
	public char buf_indic[] = new char[buf_len];
	public char buf_state[] = new char[buf_len];

	public void sendGetFastBufB(char[] buf, String req_string, SocketAddress dest, StringBuilder bd) throws IOException {
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

		
		bufferedReader.read(buf, 0, buf_len);
		

		bufferedReader.close();
		bufferedWriter.close();
		socket.close();
		bd.delete(0, bd.length());
		bd.append(buf);
	}
	public String sendGetFastBuf(char[] buf, String req_string, SocketAddress dest) throws IOException {
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

		
		int rlen = bufferedReader.read(buf, 0, buf_len);


		bufferedReader.close();
		bufferedWriter.close();
		socket.close();
		return String.valueOf(buf, 0, rlen);
		//.valueOf(buf);
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

		StringBuilder contentBuf = new StringBuilder();
//		bufferedReader.read()
		
		// TODO: 优化过程，一次性读取
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();
		bufferedReader.readLine();

		while ((line = bufferedReader.readLine()) != null) {
			contentBuf.append(line);
		}
		result = contentBuf.toString();
		bufferedReader.close();
		bufferedWriter.close();
		socket.close();
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


//			Executors.newCachedThreadPool().submit(() -> {
//				strState = sendGetFast(state_request, req_addr);
//				completableFuture0.complete(true);
//				return null;
//			});
//			
////			strIndic = sendGetFast(indic_request, req_addr);
//			strIndic = sendGetFastBuf(buf_indic, indic_request, req_addr);
//			
			Executors.newCachedThreadPool().submit(() -> {
//				sendGetFastBufB(buf_state, state_request, req_addr, strBState);
				strState = sendGetFastBuf(buf_state, state_request, req_addr);
//				System.out.println(strState);
				completableFuture0.complete(true);
				return null;
			});
//			sendGetFastBufB(buf_state, state_request, req_addr, strBIndic);
			strIndic = sendGetFastBuf(buf_indic, indic_request, req_addr);
			completableFuture0.get();
			

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
