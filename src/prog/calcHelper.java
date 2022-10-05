package prog;

public class calcHelper {
	public class simpleMovingAverage{
		double data[];
		int cnt;
		int n;
		double avg;
		public simpleMovingAverage(int num){
			data = new double[num];
			n = num;
			cnt = 0;
			avg = 0;
		}
		public double addNewData(double ndata){
			if (cnt < n) {
				// 添加数据
				data[cnt++] = ndata;
				double tavg = 0;
				for (int i = 0; i < cnt; i++){
					tavg += data[i];
				}
				avg = tavg/cnt;
//				return 0;
			}
			else{
				int ridx = cnt++ % n; 
				avg = avg + (ndata - data[ridx])/n;
				data[ridx] = ndata;
			}
			return avg;
		}
	}
}
