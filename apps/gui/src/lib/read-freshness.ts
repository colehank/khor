/** An unconfirmed read says nothing about whether the remote task ended. */
export class ReadFreshness{
 private confirmed:number;
 private failures=0;
 constructor(now:number){this.confirmed=now;}
 success(now:number){this.confirmed=now;this.failures=0;}
 failure(){this.failures++;}
 stale(now:number){return now-this.confirmed>=10000||(this.failures>=3&&now-this.confirmed>=1500);}
}
